// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Contract selection by provider crate (card **W44-QUIRIN-AUDIT-PROVIDERS**).
//!
//! A provider is a `quirin-provider-*` or `qcore-provider-*` crate on the
//! published contract load path (`providers/contract-load-path.json`), read
//! as **data**: this crate links no provider. A selection entry is either
//! `NAME` — the crate's declared **resource view** (card
//! **W44-PROVIDER-RESOURCE-VIEW**): the owned modules the resource-contract
//! loader accepts, generated into the load path by `providers/contracts.py
//! sync` and checked by `check` against [`probe_resource_views`] — or
//! `NAME:MODULE`, one owned module, spelled exactly like an `#include`
//! target. Entries keep their order; a module selected twice refuses. A crate
//! whose load-path record declares no view refuses as a whole-crate entry; a
//! crate whose view is empty contributes no module, and a selection that
//! resolves to no module refuses.
//!
//! **All loaded** ([`resolve_all_loaded`]): in contracts-only mode, with no
//! selection and no explicit manifest, every crate on the load path is
//! selected whole, in load-path order. This is the one default provider set,
//! recorded as `"selection": "all-loaded"`; it exists only in contracts-only
//! mode, and an explicit selection narrows it.
//!
//! Resolution is total-or-refuse. The load path itself is validated first
//! (namespace derivation and collisions, dependency cycles, missing
//! dependencies, listed modules that are not on disk, stale publication pins).
//! Then an unknown crate, a module the crate does not own, a duplicate module
//! and an empty selection refuse, and the opt-in `quirin_providers_only` guard
//! refuses any `qcore-provider-*` crate in the dependency closure.
//!
//! The **dependency closure** of the selected crates is resolved and recorded
//! (name, namespace, version, declared dependencies, publication pins). It is
//! the resolution scope of the selected modules' `#include` directives, which
//! the existing elaborator follows one declared edge at a time. A dependency's
//! own modules are not selected by being a dependency: a contract-use edge
//! means "my contracts inherit from yours", provider crates are data that
//! nothing loads implicitly, and a selection that pulled whole dependencies in
//! could not express a manifest's module subset.
//!
//! The selection is **lowered** to the schema-2 resource manifest a person
//! would write beside the load path's directory — a file path for a module
//! that uses no inclusion (so it loads exactly as a schema-1 entry does), a
//! `crate:module` reference for one that does — and that text is loaded by
//! the unchanged manifest loader. A selection equal to a hand-written
//! manifest therefore loads the same environment, keyed the same way.
//!
//! **User load paths (card W45-USER-CONTRACT-PLUGINS).** A load path that
//! extends a shipped one (`qcore_contract::load_path_manifest`) adds the
//! user's own crates. Every selected module is then lowered as a
//! `crate:module` reference, since a shipped crate's files do not sit beside
//! the user's manifest; a user module that declares a function a shipped crate
//! also declares refuses unless the manifest overrides it explicitly, and each
//! override is recorded ([`crate::user_plugins`]).
//!
//! Lean model: `proof/QCore/ProviderSelection.lean`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use qcore_contract::contract_inclusion::{
    CONTRACT_CRATE_PREFIX, EXECUTABLE_CONTRACT_CRATE_PREFIX, uses_contract_inclusion,
};
use qcore_contract::load_path_manifest::{
    LoadPathExtension, LoadPathManifest, load_contract_load_path_manifest,
};
use qcore_schemas::sha256_hex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Where a selection was declared. Both may declare one run; see
/// [`agree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelectionSource {
    CommandLine,
    Configuration,
    /// The contracts-only all-loaded default: no form declared a selection.
    Default,
}

impl SelectionSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::CommandLine => "command_line",
            Self::Configuration => "configuration",
            Self::Default => "default",
        }
    }
}

/// One selected module, in selection order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedModule {
    pub crate_name: String,
    pub module: String,
    pub sha256: String,
    /// Whether the module uses `#include`/`contract inherit`, and so is
    /// lowered to a `crate:module` reference rather than a file path.
    pub uses_inclusion: bool,
}

/// How a selection was made: named entries, or every crate on the load path
/// (the contracts-only default).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionKind {
    Explicit,
    AllLoaded,
}

impl SelectionKind {
    /// The recorded spelling (`selection` in the sidecar and the invocation
    /// record).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::AllLoaded => "all-loaded",
        }
    }
}

/// A resolved provider selection: its lowered manifest and its record.
#[derive(Clone, Debug)]
pub struct ResolvedProviders {
    kind: SelectionKind,
    load_path: PathBuf,
    entries: Vec<String>,
    modules: Vec<SelectedModule>,
    closure: Vec<ClosureCrate>,
    quirin_providers_only: bool,
    manifest_path: PathBuf,
    manifest_text: String,
    load_path_sha256: String,
    sources: BTreeSet<SelectionSource>,
    /// Card W45-USER-CONTRACT-PLUGINS: the user load path's additions and the
    /// overrides this selection applies; `None` for a shipped load path.
    user: Option<(LoadPathExtension, Value)>,
}

#[derive(Clone, Debug, Serialize)]
struct ClosureCrate {
    name: String,
    namespace: String,
    version: String,
    role: &'static str,
    dependencies: Vec<String>,
    contracts: String,
    resource_view: Option<Vec<String>>,
    selected_modules: BTreeMap<String, String>,
    published: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct LoweredManifest<'a> {
    schema_version: u32,
    load_path: &'a str,
    modules: Vec<String>,
}

fn provider_prefix(name: &str) -> bool {
    name.starts_with(CONTRACT_CRATE_PREFIX) || name.starts_with(EXECUTABLE_CONTRACT_CRATE_PREFIX)
}

/// Resolve `entries` against the load-path manifest at `load_path`.
///
/// `load_path` is used as spelled (a relative spelling is the caller's to
/// anchor): the lowered manifest sits in its grandparent directory, so the
/// selected files are keyed by the path reached through it, as a manifest
/// beside `providers/` keys them.
pub fn resolve_providers(
    load_path: &Path,
    entries: &[String],
    quirin_providers_only: bool,
    source: SelectionSource,
) -> Result<ResolvedProviders, String> {
    resolve_provider_entries(load_path, entries, quirin_providers_only, source, false)
}

fn resolve_provider_entries(
    load_path: &Path,
    entries: &[String],
    quirin_providers_only: bool,
    source: SelectionSource,
    deduplicate_cross_library_modules: bool,
) -> Result<ResolvedProviders, String> {
    if entries.is_empty() {
        return Err("provider selection is empty: name at least one provider crate".into());
    }
    let directory = load_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or("the contract load path must name a file inside a directory")?;
    let base = directory
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or("the contract load path's directory must itself have a parent directory")?;
    let directory_name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("the contract load path's directory name is not UTF-8")?;
    let file_name = load_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("the contract load path's file name is not UTF-8")?;
    let load_path_bytes = fs::read(load_path)
        .map_err(|error| format!("contract load path {}: {error}", load_path.display()))?;
    let manifest: LoadPathManifest =
        load_contract_load_path_manifest(load_path).map_err(|error| error.to_string())?;
    // Card W45-USER-CONTRACT-PLUGINS: a shipped crate of a user load path has
    // its root beside the extended manifest, not beside this one.
    let shipped_directory = manifest
        .extension
        .as_ref()
        .and_then(|extension| extension.base.parent().map(Path::to_path_buf));
    let known = || {
        manifest
            .contract_roots
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut modules = Vec::new();
    let mut seen = BTreeSet::new();
    let mut selected_crates = BTreeSet::new();
    for entry in entries {
        let (crate_name, module) = match entry.split_once(':') {
            Some((crate_name, module)) => (crate_name, Some(module)),
            None => (entry.as_str(), None),
        };
        if !provider_prefix(crate_name) {
            return Err(format!(
                "`{entry}` is not a provider: a provider is a `{CONTRACT_CRATE_PREFIX}*` or \
                 `{EXECUTABLE_CONTRACT_CRATE_PREFIX}*` crate on the contract load path"
            ));
        }
        let krate = manifest.load_path.crate_named(crate_name).ok_or_else(|| {
            format!(
                "unknown provider `{crate_name}`: it is not on the contract load path {} \
                 (providers: {})",
                load_path.display(),
                known()
            )
        })?;
        let wanted: Vec<&String> = match module {
            None => manifest
                .resource_views
                .get(crate_name)
                .ok_or_else(|| {
                    format!(
                        "provider `{crate_name}` declares no resource view on the contract load \
                         path {}, so it cannot be selected whole; regenerate the load path \
                         (`python3 providers/contracts.py sync`) or name modules as \
                         `{crate_name}:MODULE`",
                        load_path.display()
                    )
                })?
                .iter()
                .collect(),
            Some(module) => vec![
                krate
                    .modules
                    .get_key_value(module)
                    .map(|(key, _)| key)
                    .ok_or_else(|| {
                        format!("provider `{crate_name}` owns no contract module `{module}`")
                    })?,
            ],
        };
        selected_crates.insert(crate_name.to_owned());
        for module in wanted {
            if !seen.insert((crate_name.to_owned(), module.clone())) {
                if deduplicate_cross_library_modules {
                    continue;
                }
                return Err(format!(
                    "contract module `{crate_name}:{module}` is selected twice"
                ));
            }
            let text = &krate.modules[module];
            modules.push(SelectedModule {
                crate_name: crate_name.to_owned(),
                module: module.clone(),
                sha256: sha256_hex(text.as_bytes()),
                uses_inclusion: uses_contract_inclusion(text),
            });
        }
    }
    if modules.is_empty() {
        return Err(format!(
            "provider selection {entries:?} resolves to no resource contract module: the \
             resource view of every crate it names whole is empty"
        ));
    }
    // The dependency closure, breadth-first over declared edges. The load path
    // has already refused a missing dependency and a cycle.
    let mut closure = BTreeMap::new();
    let mut pending: Vec<String> = selected_crates.iter().cloned().collect();
    while let Some(name) = pending.pop() {
        if closure.contains_key(&name) {
            continue;
        }
        let krate = manifest
            .load_path
            .crate_named(&name)
            .ok_or_else(|| format!("dependency `{name}` is not on the contract load path"))?;
        pending.extend(krate.dependencies.iter().cloned());
        let root = manifest
            .contract_roots
            .get(&name)
            .and_then(|root| {
                root.strip_prefix(directory)
                    .ok()
                    .or_else(|| root.strip_prefix(shipped_directory.as_deref()?).ok())
            })
            .map(crate::repository_path)
            .ok_or_else(|| format!("provider `{name}` has no contract root"))?;
        closure.insert(
            name.clone(),
            ClosureCrate {
                name: name.clone(),
                namespace: krate.namespace.clone(),
                version: krate.version.clone(),
                role: if selected_crates.contains(&name) {
                    "selected"
                } else {
                    "dependency"
                },
                dependencies: krate.dependencies.iter().cloned().collect(),
                contracts: root,
                resource_view: manifest.resource_views.get(&name).cloned(),
                selected_modules: modules
                    .iter()
                    .filter(|module| module.crate_name == name)
                    .map(|module| (module.module.clone(), module.sha256.clone()))
                    .collect(),
                published: krate
                    .published
                    .iter()
                    .map(|module| (module.clone(), sha256_hex(krate.modules[module].as_bytes())))
                    .collect(),
            },
        );
    }
    if quirin_providers_only
        && let Some(name) = closure
            .keys()
            .find(|name| !name.starts_with(CONTRACT_CRATE_PREFIX))
    {
        return Err(format!(
            "--quirin-providers-only refuses `{name}`: the resolved provider closure may contain \
             only `{CONTRACT_CRATE_PREFIX}*` crates"
        ));
    }
    let user = match &manifest.extension {
        Some(extension) => Some((
            extension.clone(),
            crate::user_plugins::check_overrides(&manifest, extension, &modules)?,
        )),
        None => None,
    };
    let lowered = LoweredManifest {
        schema_version: 2,
        load_path: &format!("{directory_name}/{file_name}"),
        modules: modules
            .iter()
            .map(|module| {
                // Card W45-USER-CONTRACT-PLUGINS: on a user load path every
                // module is a `crate:module` reference.
                if module.uses_inclusion || user.is_some() {
                    format!("{}:{}", module.crate_name, module.module)
                } else {
                    format!(
                        "{directory_name}/{}/{}",
                        closure[&module.crate_name].contracts, module.module
                    )
                }
            })
            .collect(),
    };
    let mut manifest_text =
        serde_json::to_string_pretty(&lowered).map_err(|error| error.to_string())?;
    manifest_text.push('\n');
    Ok(ResolvedProviders {
        kind: SelectionKind::Explicit,
        load_path: load_path.to_path_buf(),
        entries: entries.to_vec(),
        modules,
        closure: closure.into_values().collect(),
        quirin_providers_only,
        manifest_path: base.join("provider-selection.json"),
        manifest_text,
        load_path_sha256: sha256_hex(&load_path_bytes),
        sources: BTreeSet::from([source]),
        user,
    })
}

/// The contracts-only default (card **W44-PROVIDER-RESOURCE-VIEW**): every
/// crate on the load path, selected whole (its resource view), in load-path
/// order, recorded as `"selection": "all-loaded"`. Nothing else is a default
/// provider set; a full-mode run with no selection still refuses.
pub fn resolve_all_loaded(
    load_path: &Path,
    quirin_providers_only: bool,
) -> Result<ResolvedProviders, String> {
    let manifest =
        load_contract_load_path_manifest(load_path).map_err(|error| error.to_string())?;
    let entries: Vec<String> = manifest.contract_roots.keys().cloned().collect();
    let mut resolved = resolve_providers(
        load_path,
        &entries,
        quirin_providers_only,
        SelectionSource::Default,
    )?;
    resolved.kind = SelectionKind::AllLoaded;
    Ok(resolved)
}

/// The resource view the resource-contract loader admits for every crate on
/// a load path (card **W44-PROVIDER-RESOURCE-VIEW**), computed by loading:
/// each owned module alone, as `quirin-audit` would select it
/// (`NAME:MODULE`, lowered and loaded by the unchanged manifest loader); then
/// each crate's accepted modules together; then every crate's together, which
/// is the all-loaded environment. A module is in its crate's view exactly
/// when it loads alone; a view or the union that does not load together
/// refuses the probe. Declared views are not read: this computes them, and
/// `providers/contracts.py` writes (`sync`) or compares (`check`) the result.
///
/// Card **CHUNKIO-META-LEN-BOUNDS-CONTRACTS**: the same probe computes each
/// crate's **bounds view**, the owned modules the bounds contract v1 loader
/// accepts alone (keyed `crate:module`, as the audit routes load them); every
/// bounds view must also load together. The two loaders reject each other's
/// vocabulary, so the views are disjoint.
pub fn probe_resource_views(load_path: &Path) -> Result<Value, String> {
    let load_path = &std::path::absolute(load_path).map_err(|error| error.to_string())?;
    let manifest =
        load_contract_load_path_manifest(load_path).map_err(|error| error.to_string())?;
    // Resolution must succeed (a failure is the probe's, never a module's);
    // only the loader's verdict on the resolved selection is recorded.
    let resolve =
        |entries: &[String]| resolve_providers(load_path, entries, false, SelectionSource::Default);
    let load = |entries: &[String]| -> Result<Result<(), String>, String> {
        Ok(ContractInput::providers(&resolve(entries)?, None)
            .load()
            .map(|_| ()))
    };
    let mut crates = serde_json::Map::new();
    let mut union = Vec::new();
    let mut bounds_union = BTreeMap::new();
    let mut outer_union = BTreeMap::new();
    // Card W45-USER-CONTRACT-PLUGINS: on a user load path only the user's own
    // crates are probed; the extended load path's views are its own.
    let probed = manifest.contract_roots.keys().filter(|name| {
        manifest
            .extension
            .as_ref()
            .is_none_or(|extension| extension.user_crates.contains(*name))
    });
    for name in probed {
        let krate = manifest
            .load_path
            .crate_named(name)
            .ok_or_else(|| format!("provider `{name}` is not on the contract load path"))?;
        let mut view = Vec::new();
        let mut refused = serde_json::Map::new();
        for module in krate.modules.keys() {
            let entry = format!("{name}:{module}");
            match load(std::slice::from_ref(&entry))? {
                Ok(()) => view.push(entry),
                Err(error) => {
                    refused.insert(module.clone(), json!(error));
                }
            }
        }
        if !view.is_empty() {
            load(&view)?.map_err(|error| {
                format!("the resource view of `{name}` does not load together: {error}")
            })?;
        }
        let modules: Vec<String> = view
            .iter()
            .map(|entry| entry[name.len() + 1..].to_owned())
            .collect();
        union.extend(view);
        // Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the bounds loader's view.
        let mut bounds_view = Vec::new();
        for (module, text) in &krate.modules {
            let key = format!("{name}:{module}");
            let alone = BTreeMap::from([(key.clone(), text.clone())]);
            if qcore_contract::bounds_contract::load_bounds_contract_sources(&alone).is_ok() {
                bounds_view.push(module.clone());
                bounds_union.insert(key, text.clone());
            }
        }
        if let Some(both) = bounds_view.iter().find(|module| modules.contains(*module)) {
            return Err(format!(
                "`{name}:{both}` is accepted by both the resource and the bounds loader"
            ));
        }
        // Card OUTER-CONTRACT: the outer contract loader's view: every owned
        // module, in neither other view, that names the selector, loaded
        // together (a citation needs its definition, so a module is never
        // probed alone). A set that does not load refuses the probe rather
        // than leaving a loader's runtime condition out of every view.
        let mut outer_view = Vec::new();
        let mut outer_sources = BTreeMap::new();
        for (module, text) in &krate.modules {
            if text.contains(qcore_contract::outer_contract::OUTER_CONTRACT_MARKER)
                && !modules.contains(module)
                && !bounds_view.contains(module)
            {
                outer_view.push(module.clone());
                outer_sources.insert(format!("{name}:{module}"), text.clone());
            }
        }
        if !outer_sources.is_empty() {
            qcore_contract::outer_contract::load_outer_contract_sources(&outer_sources).map_err(
                |error| format!("the outer modules of `{name}` do not load together: {error}"),
            )?;
            outer_union.extend(outer_sources);
        }
        crates.insert(
            name.clone(),
            json!({"resource_view": modules, "refused": refused, "bounds_view": bounds_view, "outer_view": outer_view}),
        );
    }
    // Card OUTER-CONTRACT: the outer views load together.
    if !outer_union.is_empty() {
        qcore_contract::outer_contract::load_outer_contract_sources(&outer_union).map_err(
            |error| format!("the outer views of the load path do not load together: {error}"),
        )?;
    }
    if !bounds_union.is_empty() {
        qcore_contract::bounds_contract::load_bounds_contract_sources(&bounds_union).map_err(
            |error| format!("the bounds views of the load path do not load together: {error}"),
        )?;
    }
    if !union.is_empty() {
        load(&union)?.map_err(|error| {
            format!("the resource views of the load path do not load together: {error}")
        })?;
    }
    Ok(json!({
        "schema_version": 1,
        "load_path": load_path,
        "load_path_sha256": sha256_hex(&fs::read(load_path).map_err(|error| error.to_string())?),
        "crates": crates,
        "rule": "a module is in its crate's resource view exactly when quirin-audit's NAME:MODULE selection of it, lowered and loaded by the unchanged resource-contract manifest loader, is accepted; each view, and all views together, must also load; a module is in its crate's bounds view exactly when the bounds contract v1 loader accepts it alone, and all bounds views must load together",
    }))
}

/// The both-forms rule: a selection declared on the command line **and** in a
/// configuration file is admitted only when the two declare the same run —
/// the same load path spelling, the same guard and the same lowered manifest,
/// so the same modules in the same order. Anything else is a conflict and
/// refuses; nothing is merged, and the record names both sources.
pub fn agree(
    command_line: ResolvedProviders,
    configuration: ResolvedProviders,
) -> Result<ResolvedProviders, String> {
    if command_line.load_path != configuration.load_path {
        return Err(format!(
            "provider selection conflict: the command line resolves providers on {} and the \
             configuration on {}",
            command_line.load_path.display(),
            configuration.load_path.display()
        ));
    }
    if command_line.quirin_providers_only != configuration.quirin_providers_only {
        return Err(
            "provider selection conflict: the command line and the configuration disagree on \
             quirin_providers_only"
                .into(),
        );
    }
    if command_line.manifest_text != configuration.manifest_text {
        return Err(format!(
            "provider selection conflict: the command line selects {:?} and the configuration \
             selects {:?}, which resolve to different contract modules; give one, or make them \
             equal",
            command_line.entries, configuration.entries
        ));
    }
    let mut merged = command_line;
    merged.sources.extend(configuration.sources);
    Ok(merged)
}

impl ResolvedProviders {
    /// Where the lowered manifest is taken to sit; relative entries resolve
    /// against its parent.
    #[must_use]
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    /// The lowered schema-2 manifest.
    #[must_use]
    pub fn manifest_text(&self) -> &str {
        &self.manifest_text
    }

    #[must_use]
    pub fn modules(&self) -> &[SelectedModule] {
        &self.modules
    }

    /// Named entries, or the contracts-only all-loaded default.
    #[must_use]
    pub fn kind(&self) -> SelectionKind {
        self.kind
    }

    /// Card W45-USER-CONTRACT-PLUGINS: the user load path's additions, when
    /// the selection was resolved on one.
    #[must_use]
    pub fn extension(&self) -> Option<&LoadPathExtension> {
        self.user.as_ref().map(|(extension, _)| extension)
    }

    /// Crate names of the dependency closure, sorted.
    pub fn closure_names(&self) -> impl Iterator<Item = &str> {
        self.closure.iter().map(|krate| krate.name.as_str())
    }

    /// The sidecar record: everything needed to rerun the selection and to
    /// check that a rerun read the same provider data.
    #[must_use]
    pub fn record(&self, profile_manifest: Option<&Path>) -> Value {
        let mut record = json!({
            "selection_version": 1,
            "selection": self.kind.as_str(),
            "sources": self.sources.iter().map(|source| source.as_str()).collect::<Vec<_>>(),
            "entries": self.entries,
            "load_path": self.load_path,
            "load_path_sha256": self.load_path_sha256,
            "quirin_providers_only": self.quirin_providers_only,
            "crates": self.closure,
            "modules": self.modules.iter().map(|module| json!({
                "crate": module.crate_name, "module": module.module,
                "sha256": module.sha256,
                "loaded_as": if module.uses_inclusion || self.user.is_some() { "reference" } else { "path" },
            })).collect::<Vec<_>>(),
            "lowered_manifest_sha256": sha256_hex(self.manifest_text.as_bytes()),
            "rule": "each entry is NAME (the crate's declared resource view: the owned modules the resource-contract loader accepts, generated and checked by providers/contracts.py, in load-path order) or NAME:MODULE; with no selection, a contracts-only run selects every load-path crate whole (selection all-loaded); the dependency closure is resolved, recorded and is the include resolution scope, and a dependency's own modules are not selected by being a dependency; the selection is lowered to the schema-2 manifest recorded as `contract_manifest` and loaded by the unchanged manifest loader",
        });
        // Card W45-USER-CONTRACT-PLUGINS: omitted for a shipped load path.
        if let Some((_, user)) = &self.user {
            record["user_load_path"] = user.clone();
        }
        if let Some(path) = profile_manifest {
            record["profile_contract_manifest"] = json!({
                "path": path,
                "role": match self.kind {
                    SelectionKind::Explicit => "declared by the source profile as its default; not read: an explicit provider selection supersedes a profile default",
                    SelectionKind::AllLoaded => "declared by the source profile as its default; not read: a contracts-only run with no selection loads all contracts, which supersedes a profile default",
                },
            });
        }
        record
    }
}

/// The resource-contract input of an audit: a manifest file, or a provider
/// selection lowered to a manifest. Either way one text is loaded by
/// `load_resource_contract_manifest_text`, digested into every compile-task
/// identity and recorded as the sidecar's `contract_manifest`.
#[derive(Clone, Debug)]
pub struct ContractInput {
    manifest_path: PathBuf,
    manifest_text: String,
    providers: Option<Value>,
    /// Card W45-USER-CONTRACT-PLUGINS: a user load path's additions.
    extension: Option<LoadPathExtension>,
}

impl ContractInput {
    /// A manifest file, read once.
    pub fn manifest(path: &Path) -> Result<Self, String> {
        Ok(Self {
            manifest_path: path.to_path_buf(),
            manifest_text: fs::read_to_string(path).map_err(|error| error.to_string())?,
            providers: None,
            extension: None,
        })
    }

    /// A resolved provider selection. `profile_manifest` is the source
    /// profile's declared default, recorded as superseded and never read.
    #[must_use]
    pub fn providers(resolved: &ResolvedProviders, profile_manifest: Option<&Path>) -> Self {
        Self {
            manifest_path: resolved.manifest_path().to_path_buf(),
            manifest_text: resolved.manifest_text().to_owned(),
            providers: Some(resolved.record(profile_manifest)),
            extension: resolved.extension().cloned(),
        }
    }

    #[must_use]
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    #[must_use]
    pub fn manifest_text(&self) -> &str {
        &self.manifest_text
    }

    /// The provider record for the sidecar, when providers were selected.
    #[must_use]
    pub fn provider_record(&self) -> Option<&Value> {
        self.providers.as_ref()
    }

    /// Card W45-USER-CONTRACT-PLUGINS: the user load path's additions (its
    /// checker bindings among them), when providers were resolved on one.
    #[must_use]
    pub fn extension(&self) -> Option<&LoadPathExtension> {
        self.extension.as_ref()
    }

    /// Load the environment with the unchanged manifest loader. Card
    /// CONTRACT-SUPERSESSION (contract spec §C06.6): a function more than one
    /// selected module declares is decided by the pin stamps of the locks
    /// covering them ([`crate::contract_revision::LockStamps`]); with no
    /// duplicate no stamp is read and the environment is the one loaded
    /// before.
    pub fn load(&self) -> Result<qcore_schemas::contract::ResourceContracts, String> {
        let owners = crate::contract_revision::ModuleOwners::for_manifest(
            &self.manifest_path,
            &self.manifest_text,
        )?;
        qcore_contract::contract_loader::load_resource_contract_manifest_text_stamped(
            &self.manifest_path,
            &self.manifest_text,
            &crate::contract_revision::LockStamps::new(&owners),
        )
    }
}

/// Per-library contract scope: a library's own provider entries (`NAME` or
/// `NAME:MODULE`) and the dependent libraries whose provider scopes are
/// transitively brought into scope when auditing this library slice.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryProviderScope {
    /// Provider entries (`NAME` or `NAME:MODULE`) owned or required directly
    /// by this library slice.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<String>,
    /// Names of dependent libraries whose `providers` (and transitive
    /// `dependent_libraries`) are also loaded when auditing this library.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependent_libraries: Vec<String>,
}

impl LibraryProviderScope {
    /// Whether this scope declares any per-library provider or dependent-library
    /// entry.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty() && self.dependent_libraries.is_empty()
    }
}

const fn default_library_providers_schema_version() -> u32 {
    1
}

/// A standalone per-library provider scoping document (`--library-providers FILE`
/// or `<profile_directory>/library-providers.json`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryProvidersFile {
    #[serde(default = "default_library_providers_schema_version")]
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_load_path: Option<PathBuf>,
    #[serde(default)]
    pub quirin_providers_only: bool,
    pub libraries: BTreeMap<String, LibraryProviderScope>,
}

impl LibraryProvidersFile {
    /// Load and validate the document structure of a per-library provider
    /// scoping file. Accepts either `{ "schema_version": 1, "libraries": { ... } }`
    /// or a direct `{ "<library>": { "providers": [...], "dependent_libraries": [...] } }`
    /// map.
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        Self::parse(path, &bytes)
    }

    pub fn parse(path: &Path, bytes: &[u8]) -> Result<Self, String> {
        let at = |message: String| format!("{}: {message}", path.display());
        let raw: Value = serde_json::from_slice(bytes).map_err(|error| at(error.to_string()))?;
        let file = if raw.get("libraries").is_some() {
            serde_json::from_value::<Self>(raw).map_err(|error| at(error.to_string()))?
        } else {
            let libraries = serde_json::from_value::<BTreeMap<String, LibraryProviderScope>>(raw)
                .map_err(|error| at(error.to_string()))?;
            Self {
                schema_version: 1,
                contract_load_path: None,
                quirin_providers_only: false,
                libraries,
            }
        };
        if file.schema_version != 1 {
            return Err(at(format!(
                "unsupported library providers schema_version {}",
                file.schema_version
            )));
        }
        if file.libraries.is_empty() {
            return Err(at(
                "library providers table is empty: declare at least one library scope".into(),
            ));
        }
        Ok(file)
    }
}

/// The resolved provider selection for one library slice, combining the
/// library's own `providers` with those of its transitive `dependent_libraries`
/// in first-visited order.
#[derive(Clone, Debug)]
pub struct ResolvedLibraryProviders {
    pub library: String,
    pub own_providers: Vec<String>,
    pub dependent_libraries: Vec<String>,
    /// Ordered transitive library closure `[library, ...dependent_libraries]`.
    pub library_closure: Vec<String>,
    pub resolved: ResolvedProviders,
    pub input: ContractInput,
}

impl ResolvedLibraryProviders {
    #[must_use]
    pub fn record(&self) -> Value {
        json!({
            "library": self.library,
            "own_providers": self.own_providers,
            "dependent_libraries": self.dependent_libraries,
            "library_closure": self.library_closure,
            "entries": self.resolved.entries,
            "modules": self.resolved.modules().iter().map(|module| json!({
                "crate": module.crate_name,
                "module": module.module,
                "sha256": module.sha256,
            })).collect::<Vec<_>>(),
            "lowered_manifest_sha256": sha256_hex(self.resolved.manifest_text().as_bytes()),
        })
    }
}

/// A resolved per-library provider scoping table for all libraries in an audit
/// run, together with the deduplicated run-wide union [`ContractInput`].
#[derive(Clone, Debug)]
pub struct ResolvedLibraryProvidersTable {
    pub load_path: PathBuf,
    pub quirin_providers_only: bool,
    pub slices: BTreeMap<String, ResolvedLibraryProviders>,
    pub union_resolved: ResolvedProviders,
    pub union_input: ContractInput,
    sources: BTreeSet<SelectionSource>,
}

impl ResolvedLibraryProvidersTable {
    /// Look up the resolved contract input for `library`.
    pub fn slice_for(&self, library: &str) -> Result<&ResolvedLibraryProviders, String> {
        self.slices.get(library).ok_or_else(|| {
            format!(
                "library provider scope is not resolved for library `{library}` (resolved: {})",
                self.slices
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
    }

    /// The sidecar record for `library_provider_selection`.
    #[must_use]
    pub fn record(&self) -> Value {
        json!({
            "selection_version": 1,
            "mode": "per-library-slice",
            "sources": self.sources.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            "load_path": self.load_path,
            "quirin_providers_only": self.quirin_providers_only,
            "libraries": self.slices.values().map(ResolvedLibraryProviders::record).collect::<Vec<_>>(),
            "union_lowered_manifest_sha256": sha256_hex(self.union_resolved.manifest_text().as_bytes()),
            "rule": "each library slice is audited under its own provider selection plus the transitive dependent_libraries closure in declared order; within a single library's providers list duplicates refuse, while shared modules across a library and its dependent_libraries are deduplicated in first-seen order; each slice's contract environment and unit outcomes are dropped after the slice is absorbed",
        })
    }
}

/// Compute the ordered transitive closure `[library, ...dependent_libraries]`,
/// refusing unknown libraries or dependency cycles.
pub fn resolve_dependent_library_closure(
    scopes: &BTreeMap<String, LibraryProviderScope>,
    library: &str,
) -> Result<Vec<String>, String> {
    let known = || {
        scopes
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !scopes.contains_key(library) {
        return Err(format!(
            "library provider scope is not declared for library `{library}` (declared libraries: {})",
            known()
        ));
    }
    let mut closure = Vec::new();
    let mut visited = BTreeSet::new();
    let mut visiting = Vec::<String>::new();

    fn dfs(
        current: &str,
        scopes: &BTreeMap<String, LibraryProviderScope>,
        known: &dyn Fn() -> String,
        visiting: &mut Vec<String>,
        visited: &mut BTreeSet<String>,
        closure: &mut Vec<String>,
    ) -> Result<(), String> {
        if let Some(pos) = visiting.iter().position(|item| item == current) {
            let mut cycle = visiting[pos..].to_vec();
            cycle.push(current.to_owned());
            return Err(format!(
                "library dependency cycle in library provider scopes: {}",
                cycle.join(" -> ")
            ));
        }
        if visited.contains(current) {
            return Ok(());
        }
        let scope = scopes.get(current).ok_or_else(|| {
            let owner = visiting.last().map_or("<root>", String::as_str);
            format!(
                "library `{owner}` names unknown dependent library `{current}` (declared libraries: {})",
                known()
            )
        })?;
        visiting.push(current.to_owned());
        visited.insert(current.to_owned());
        closure.push(current.to_owned());
        for dep in &scope.dependent_libraries {
            if !scopes.contains_key(dep.as_str()) {
                return Err(format!(
                    "library `{current}` names unknown dependent library `{dep}` (declared libraries: {})",
                    known()
                ));
            }
            dfs(dep, scopes, known, visiting, visited, closure)?;
        }
        visiting.pop();
        Ok(())
    }

    dfs(
        library,
        scopes,
        &known,
        &mut visiting,
        &mut visited,
        &mut closure,
    )?;
    Ok(closure)
}

/// Resolve the provider contract selection for one `library` slice from
/// `scopes`: validates each library in `[library, ...dependent_libraries]`
/// individually (so an empty or duplicate entry inside a single library's
/// `providers` refuses) and then resolves the combined slice selection with
/// cross-library module deduplication.
pub fn resolve_library_providers(
    load_path: &Path,
    scopes: &BTreeMap<String, LibraryProviderScope>,
    library: &str,
    quirin_providers_only: bool,
    source: SelectionSource,
    profile_manifest: Option<&Path>,
) -> Result<ResolvedLibraryProviders, String> {
    let library_closure = resolve_dependent_library_closure(scopes, library)?;
    let mut combined_entries = Vec::new();
    for member in &library_closure {
        let scope = &scopes[member];
        if scope.providers.is_empty() {
            return Err(format!(
                "library `{member}` declares an empty provider selection: name at least one provider crate"
            ));
        }
        // Validate each library's own provider list with strict duplicate checking.
        resolve_providers(load_path, &scope.providers, quirin_providers_only, source)
            .map_err(|error| format!("library `{member}`: {error}"))?;
        for entry in &scope.providers {
            if !combined_entries.contains(entry) {
                combined_entries.push(entry.clone());
            }
        }
    }
    let resolved = resolve_provider_entries(
        load_path,
        &combined_entries,
        quirin_providers_only,
        source,
        true,
    )?;
    let input = ContractInput::providers(&resolved, profile_manifest);
    let own = &scopes[library];
    Ok(ResolvedLibraryProviders {
        library: library.to_owned(),
        own_providers: own.providers.clone(),
        dependent_libraries: own.dependent_libraries.clone(),
        library_closure,
        resolved,
        input,
    })
}

/// Resolve per-library provider scopes for all `audited_libraries` in a run
/// upfront (fail-closed before any unit is analysed), also checking every
/// declared scope in `scopes` for cycles or invalid providers and building the
/// run-wide union [`ContractInput`].
pub fn resolve_library_providers_table(
    load_path: &Path,
    scopes: &BTreeMap<String, LibraryProviderScope>,
    audited_libraries: &[String],
    quirin_providers_only: bool,
    source: SelectionSource,
    profile_manifest: Option<&Path>,
) -> Result<ResolvedLibraryProvidersTable, String> {
    if scopes.is_empty() {
        return Err("library providers table is empty: declare at least one library scope".into());
    }
    // Validate every declared scope in `scopes` first so cycles or invalid
    // providers in helper/dependent entries are refused even if not directly
    // named as a top-level group.
    for declared in scopes.keys() {
        let _ = resolve_library_providers(
            load_path,
            scopes,
            declared,
            quirin_providers_only,
            source,
            profile_manifest,
        )?;
    }
    let mut slices = BTreeMap::new();
    let mut union_entries = Vec::new();
    for library in audited_libraries {
        let slice = resolve_library_providers(
            load_path,
            scopes,
            library,
            quirin_providers_only,
            source,
            profile_manifest,
        )?;
        for entry in &slice.resolved.entries {
            if !union_entries.contains(entry) {
                union_entries.push(entry.clone());
            }
        }
        slices.insert(library.clone(), slice);
    }
    if union_entries.is_empty() {
        for declared in scopes.keys() {
            let slice = resolve_library_providers(
                load_path,
                scopes,
                declared,
                quirin_providers_only,
                source,
                profile_manifest,
            )?;
            for entry in &slice.resolved.entries {
                if !union_entries.contains(entry) {
                    union_entries.push(entry.clone());
                }
            }
        }
    }
    let union_resolved = resolve_provider_entries(
        load_path,
        &union_entries,
        quirin_providers_only,
        source,
        true,
    )?;
    let union_input = ContractInput::providers(&union_resolved, profile_manifest);
    Ok(ResolvedLibraryProvidersTable {
        load_path: load_path.to_path_buf(),
        quirin_providers_only,
        slices,
        union_resolved,
        union_input,
        sources: BTreeSet::from([source]),
    })
}

/// The both-forms rule for per-library provider scopes: when declared in two
/// places (e.g. CLI `--library-providers` and configuration/selection), they
/// must resolve to the exact same per-slice manifests on the same load path.
pub fn agree_library_providers(
    first: ResolvedLibraryProvidersTable,
    second: ResolvedLibraryProvidersTable,
) -> Result<ResolvedLibraryProvidersTable, String> {
    if first.load_path != second.load_path {
        return Err(format!(
            "library provider selection conflict: one form resolves on {} and the other on {}",
            first.load_path.display(),
            second.load_path.display()
        ));
    }
    if first.quirin_providers_only != second.quirin_providers_only {
        return Err(
            "library provider selection conflict: forms disagree on quirin_providers_only".into(),
        );
    }
    let first_keys: Vec<&String> = first.slices.keys().collect();
    let second_keys: Vec<&String> = second.slices.keys().collect();
    if first_keys != second_keys {
        return Err(format!(
            "library provider selection conflict: one form scopes libraries {first_keys:?} and the other scopes {second_keys:?}"
        ));
    }
    for (library, first_slice) in &first.slices {
        let second_slice = &second.slices[library];
        if first_slice.resolved.manifest_text() != second_slice.resolved.manifest_text()
            || first_slice.library_closure != second_slice.library_closure
        {
            return Err(format!(
                "library provider selection conflict on library `{library}`: forms resolve to different library closures or contract modules"
            ));
        }
    }
    let mut merged = first;
    merged.sources.extend(second.sources);
    Ok(merged)
}
