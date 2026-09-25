//! Contract selection by provider crate (card **W44-QUIRIN-AUDIT-PROVIDERS**).
//!
//! A provider is a `quirin-provider-*` or `qcore-provider-*` crate on the
//! published contract load path (`providers/contract-load-path.json`), read
//! as **data**: this crate links no provider. A selection entry is either
//! `NAME` — every module the crate owns, in the load path's order — or
//! `NAME:MODULE`, one owned module, spelled exactly like an `#include`
//! target. Entries keep their order; a module selected twice refuses.
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
//! Lean model: `proof/QCore/ProviderSelection.lean`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use qcore_contract::contract_inclusion::{
    CONTRACT_CRATE_PREFIX, EXECUTABLE_CONTRACT_CRATE_PREFIX, uses_contract_inclusion,
};
use qcore_contract::load_path_manifest::{LoadPathManifest, load_contract_load_path_manifest};
use qcore_schemas::sha256_hex;
use serde::Serialize;
use serde_json::{Value, json};

/// Where a selection was declared. Both may declare one run; see
/// [`agree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelectionSource {
    CommandLine,
    Configuration,
}

impl SelectionSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::CommandLine => "command_line",
            Self::Configuration => "configuration",
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

/// A resolved provider selection: its lowered manifest and its record.
#[derive(Clone, Debug)]
pub struct ResolvedProviders {
    load_path: PathBuf,
    entries: Vec<String>,
    modules: Vec<SelectedModule>,
    closure: Vec<ClosureCrate>,
    quirin_providers_only: bool,
    manifest_path: PathBuf,
    manifest_text: String,
    load_path_sha256: String,
    sources: BTreeSet<SelectionSource>,
}

#[derive(Clone, Debug, Serialize)]
struct ClosureCrate {
    name: String,
    namespace: String,
    version: String,
    role: &'static str,
    dependencies: Vec<String>,
    contracts: String,
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
            None => krate.modules.keys().collect(),
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
        if wanted.is_empty() {
            return Err(format!("provider `{crate_name}` owns no contract module"));
        }
        selected_crates.insert(crate_name.to_owned());
        for module in wanted {
            if !seen.insert((crate_name.to_owned(), module.clone())) {
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
            .and_then(|root| root.strip_prefix(directory).ok())
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
    let lowered = LoweredManifest {
        schema_version: 2,
        load_path: &format!("{directory_name}/{file_name}"),
        modules: modules
            .iter()
            .map(|module| {
                if module.uses_inclusion {
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
        load_path: load_path.to_path_buf(),
        entries: entries.to_vec(),
        modules,
        closure: closure.into_values().collect(),
        quirin_providers_only,
        manifest_path: base.join("provider-selection.json"),
        manifest_text,
        load_path_sha256: sha256_hex(&load_path_bytes),
        sources: BTreeSet::from([source]),
    })
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
            "sources": self.sources.iter().map(|source| source.as_str()).collect::<Vec<_>>(),
            "entries": self.entries,
            "load_path": self.load_path,
            "load_path_sha256": self.load_path_sha256,
            "quirin_providers_only": self.quirin_providers_only,
            "crates": self.closure,
            "modules": self.modules.iter().map(|module| json!({
                "crate": module.crate_name, "module": module.module,
                "sha256": module.sha256,
                "loaded_as": if module.uses_inclusion { "reference" } else { "path" },
            })).collect::<Vec<_>>(),
            "lowered_manifest_sha256": sha256_hex(self.manifest_text.as_bytes()),
            "rule": "each entry is NAME (every module the crate owns, in load-path order) or NAME:MODULE; the dependency closure is resolved, recorded and is the include resolution scope, and a dependency's own modules are not selected by being a dependency; the selection is lowered to the schema-2 manifest recorded as `contract_manifest` and loaded by the unchanged manifest loader",
        });
        if let Some(path) = profile_manifest {
            record["profile_contract_manifest"] = json!({
                "path": path,
                "role": "declared by the source profile as its default; not read: an explicit provider selection supersedes a profile default",
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
}

impl ContractInput {
    /// A manifest file, read once.
    pub fn manifest(path: &Path) -> Result<Self, String> {
        Ok(Self {
            manifest_path: path.to_path_buf(),
            manifest_text: fs::read_to_string(path).map_err(|error| error.to_string())?,
            providers: None,
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

    /// Load the environment with the unchanged manifest loader.
    pub fn load(&self) -> Result<qcore_schemas::contract::ResourceContracts, String> {
        qcore_contract::contract_loader::load_resource_contract_manifest_text(
            &self.manifest_path,
            &self.manifest_text,
        )
    }
}
