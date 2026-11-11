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

//! The published contract load path, read as **data** (card
//! **CONTRACT-INHERIT-ADOPTION**).
//!
//! Core crates may not link provider crates (`compiler/tests/workspace_layering.rs`),
//! so the real [`ContractLoadPath`] is not built from the provider crates'
//! Rust constants. It is read from a generated manifest,
//! `providers/contract-load-path.json`, that `python3 providers/contracts.py
//! sync` writes from Cargo metadata (crate name, version, declared
//! dependencies) and from each crate's recorded `NAMESPACE` and `PUBLISHED`
//! constants, and that `providers/contracts.py check` and the provider crates'
//! own tests compare against their sources:
//!
//! ```json
//! {
//!   "schema_version": 1,
//!   "crates": [
//!     {
//!       "name": "quirin-provider-libc",
//!       "namespace": "libc",
//!       "version": "0.1.0",
//!       "contracts": "quirin-provider-libc/contracts",
//!       "dependencies": [],
//!       "modules": ["libc/errno.qv", "libc/memory.qv"],
//!       "published": { "libc/memory.qv": "<sha256>" },
//!       "resource_view": ["libc/errno.qv", "libc/memory.qv"]
//!     }
//!   ]
//! }
//! ```
//!
//! Reading it is total-or-refuse (`QI014`): the manifest's shape, every
//! crate's `contracts` directory (relative, no traversal, a real directory),
//! the owned module list (sorted, unique, admitted spellings, **exactly** the
//! `.qcore` files under that directory), and every published module's
//! SHA-256 **pin**, which must equal the text on disk. A stale manifest is
//! therefore refused rather than silently read: a parent that changed without
//! the manifest being regenerated cannot be inherited from. The validated
//! records then go through [`ContractLoadPath::new`], which applies the
//! namespace derivation (`QI017`) and the dependency rules (`QI014`).
//!
//! **Resource view (card W44-PROVIDER-RESOURCE-VIEW).** A crate record may
//! carry `resource_view`: the owned modules the resource-contract loader
//! accepts, which is what `quirin-audit --provider NAME` selects for a whole
//! crate. `providers/contracts.py sync` generates it by loading each owned
//! module through `quirin-audit`'s own selection and loader, and `check`
//! refuses a view that declares a module the loader refuses or omits one it
//! accepts. This reader checks its shape only (sorted, unique, owned); an
//! absent view is not a default, and a whole-crate selection refuses it.
//!
//! **Bounds view (card CHUNKIO-META-LEN-BOUNDS-CONTRACTS).** A crate record
//! may also carry `bounds_view`: the owned modules the bounds contract v1
//! loader accepts (`qcore_contract::bounds_contract`), generated and checked
//! the same way. The audit routes load every bounds view when the bounds
//! property is selected. This reader checks its shape (sorted, unique, owned)
//! and that no module is in both views: the two loaders reject each other's
//! vocabulary, so a module in both is a stale or hand-edited manifest.
//!
//! **Outer view (card OUTER-CONTRACT).** A crate record may also carry
//! `outer_view`: the owned modules the outer contract v1 loader accepts
//! (`qcore_contract::outer_contract`), generated and checked the same way and
//! disjoint from both other views. The audit routes load every outer view
//! when the runtime projection is selected (`QCORE_OUTER_PROJECTION`).
//!
//! **User load paths (card W45-USER-CONTRACT-PLUGINS).** A user keeps their
//! own contract crates outside this repository, laid out like the
//! `quirin-provider-*` crates, and lists them in their own load-path manifest
//! that **extends** a shipped one:
//!
//! ```json
//! {
//!   "schema_version": 1,
//!   "extends": "/opt/quirin/providers/contract-load-path.json",
//!   "crates": [ { "name": "quirin-provider-acme", "…": "…" } ],
//!   "overrides": [],
//!   "checkers": [ { "crate": "quirin-provider-acme", "module": "acme/pool.qv",
//!                   "functions": ["pool_get"], "package": "checkers/pool-check" } ]
//! }
//! ```
//!
//! The extended manifest is read by this same function, in full, and must not
//! itself extend (one level). Its crates keep their own roots; the user's
//! crates obey every rule above, and the two sets are validated together by
//! [`ContractLoadPath::new`], so a user crate can depend on a shipped one and
//! can never share a name or a namespace with one. `extends` may be absolute
//! or relative to this file and may leave its directory: it names another
//! manifest, validated in full, not a contract tree. `overrides` and
//! `checkers` are admitted only in an extending manifest, and each names one of
//! its own crates ([`LoadPathExtension`]).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use qcore_frontend::contract_inclusion::valid_contract_module_path;
use qcore_schemas::sha256_hex;
use serde::Deserialize;

use crate::contract_inclusion::{
    ContractInclusionError, ContractLoadPath, InclusionCode, PublishedContractCrate,
};

/// The only load-path manifest schema this reader accepts.
pub const LOAD_PATH_MANIFEST_SCHEMA: u32 = 1;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    crates: Vec<CrateRecord>,
    #[serde(default)]
    extends: Option<String>,
    #[serde(default)]
    overrides: Option<Vec<OverrideRecord>>,
    #[serde(default)]
    checkers: Option<Vec<CheckerRecord>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OverrideRecord {
    function: String,
    #[serde(rename = "crate")]
    user_crate: String,
    shipped_crate: String,
    reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckerRecord {
    #[serde(rename = "crate")]
    user_crate: String,
    module: String,
    functions: Vec<String>,
    package: String,
}

/// Card W45-USER-CONTRACT-PLUGINS: an explicit, reported override — a user
/// crate's contract for a function a shipped crate also declares. Without one
/// such a collision is refused by the audit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractOverride {
    pub function: String,
    pub user_crate: String,
    pub shipped_crate: String,
    pub reason: String,
}

/// Card W45-USER-CONTRACT-PLUGINS: an out-of-process checker attached to the
/// contracts of `functions` in `module` of `user_crate`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckerBinding {
    pub user_crate: String,
    /// The user crate's namespace and version, as the load path records them.
    pub crate_namespace: String,
    pub crate_version: String,
    pub module: String,
    /// SHA-256 of the module's text on the load path.
    pub module_sha256: String,
    pub functions: Vec<String>,
    /// The package directory as written, relative to the manifest.
    pub package_spelling: String,
    /// The package directory, resolved against the manifest's directory.
    pub package: PathBuf,
}

/// Card W45-USER-CONTRACT-PLUGINS: what an extending (user) manifest adds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadPathExtension {
    /// The extended (shipped) manifest, as resolved.
    pub base: PathBuf,
    /// SHA-256 of the extended manifest's bytes.
    pub base_sha256: String,
    /// The extended manifest's crates.
    pub shipped_crates: BTreeSet<String>,
    /// This manifest's own crates.
    pub user_crates: BTreeSet<String>,
    pub overrides: Vec<ContractOverride>,
    pub checkers: Vec<CheckerBinding>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CrateRecord {
    name: String,
    namespace: String,
    version: String,
    contracts: String,
    dependencies: Vec<String>,
    modules: Vec<String>,
    published: BTreeMap<String, String>,
    #[serde(default)]
    resource_view: Option<Vec<String>>,
    /// Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS.
    #[serde(default)]
    bounds_view: Option<Vec<String>>,
    /// Card OUTER-CONTRACT.
    #[serde(default)]
    outer_view: Option<Vec<String>>,
}

/// A load path read from its manifest, with the directory each crate's
/// contracts live in (so a caller can map a contract root to its crate).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadPathManifest {
    /// The validated load path.
    pub load_path: ContractLoadPath,
    /// Crate name to the directory its logical module paths are relative to.
    pub contract_roots: BTreeMap<String, PathBuf>,
    /// Crate name to its declared resource view (card
    /// W44-PROVIDER-RESOURCE-VIEW), sorted owned modules. A crate that
    /// declares none is absent.
    pub resource_views: BTreeMap<String, Vec<String>>,
    /// Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: crate name to its declared
    /// bounds view, sorted owned modules disjoint from its resource view. A
    /// crate that declares none is absent.
    pub bounds_views: BTreeMap<String, Vec<String>>,
    /// Card OUTER-CONTRACT: crate name to its declared outer view, sorted
    /// owned modules disjoint from its resource and bounds views. A crate
    /// that declares none is absent.
    pub outer_views: BTreeMap<String, Vec<String>>,
    /// Card W45-USER-CONTRACT-PLUGINS: present exactly for a manifest that
    /// extends a shipped one.
    pub extension: Option<LoadPathExtension>,
}

impl LoadPathManifest {
    /// The crate whose contract root is `directory`, if any. Directories are
    /// compared after canonicalisation, so two spellings of one directory name
    /// one crate.
    #[must_use]
    pub fn crate_for_root(&self, directory: &Path) -> Option<&str> {
        let wanted = directory.canonicalize().ok()?;
        self.contract_roots
            .iter()
            .find(|(_, root)| root.canonicalize().ok().as_deref() == Some(wanted.as_path()))
            .map(|(name, _)| name.as_str())
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

/// Whether `bytes` are the pinned content. On Windows a checkout may have
/// translated the pinned LF text to CRLF; only bytes that recover the exact
/// pinned content after that translation are accepted there (the rule
/// `bundle::ContractBundle` applies to package inventories).
fn matches_pin(bytes: &[u8], pin: &str) -> bool {
    if sha256_hex(bytes) == pin {
        return true;
    }
    if !cfg!(windows) {
        return false;
    }
    let normalized = bytes
        .split(|byte| *byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .collect::<Vec<_>>();
    sha256_hex(&normalized.join(&b'\n')) == pin
}

fn relative_directory(value: &str) -> bool {
    let path = Path::new(value);
    !value.is_empty()
        && !value.contains('\\')
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// Every `.qcore` or `.qv` file under `root`, as logical paths. Refuses a symbolic
/// link and any other non-regular entry, so nothing outside the tree is read.
fn contract_files(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut found = BTreeSet::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries =
            fs::read_dir(&directory).map_err(|e| format!("{}: {e}", directory.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            let path = entry.path();
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() {
                if path
                    .extension()
                    .is_some_and(|ext| ext == "qcore" || ext == "qv")
                {
                    let logical = path
                        .strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .replace('\\', "/");
                    found.insert(logical);
                }
            } else {
                return Err(format!(
                    "{}: not a regular file or directory",
                    path.display()
                ));
            }
        }
    }
    crate::extension_compatibility::reject_alias_collisions(found.iter().map(String::as_str))?;
    Ok(found)
}

/// Read and validate a load-path manifest (see the module documentation).
/// Relative `contracts` directories resolve against the manifest's own
/// directory.
pub fn load_contract_load_path_manifest(
    path: &Path,
) -> Result<LoadPathManifest, ContractInclusionError> {
    read_manifest(path, true)
}

fn read_manifest(
    path: &Path,
    may_extend: bool,
) -> Result<LoadPathManifest, ContractInclusionError> {
    let origin = path.display().to_string();
    let refuse = |subject: &str, message: String| {
        ContractInclusionError::new(InclusionCode::Qi014, subject, message)
    };
    let metadata = fs::symlink_metadata(path).map_err(|e| refuse(&origin, e.to_string()))?;
    if !metadata.is_file() {
        return Err(refuse(
            &origin,
            "the load-path manifest is not a regular file".into(),
        ));
    }
    let text = fs::read_to_string(path).map_err(|e| refuse(&origin, e.to_string()))?;
    let manifest: Manifest = serde_json::from_str(&text)
        .map_err(|e| refuse(&origin, format!("malformed load-path manifest: {e}")))?;
    if manifest.schema_version != LOAD_PATH_MANIFEST_SCHEMA || manifest.crates.is_empty() {
        return Err(refuse(
            &origin,
            "unsupported load-path manifest schema or no crates".into(),
        ));
    }
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    // Card W45-USER-CONTRACT-PLUGINS: an extending manifest reads its base in
    // full first; `overrides` and `checkers` exist only beside `extends`.
    let extended = match &manifest.extends {
        Some(_) if !may_extend => {
            return Err(refuse(
                &origin,
                "an extended load-path manifest must not itself extend another".into(),
            ));
        }
        Some(spelling) => {
            let base_path = base.join(spelling);
            let bytes = fs::read(&base_path)
                .map_err(|e| refuse(&origin, format!("{}: {e}", base_path.display())))?;
            Some((
                read_manifest(&base_path, false)?,
                base_path,
                sha256_hex(&bytes),
            ))
        }
        None if manifest.overrides.is_some() || manifest.checkers.is_some() => {
            return Err(refuse(
                &origin,
                "`overrides` and `checkers` belong to a load-path manifest that extends another"
                    .into(),
            ));
        }
        None => None,
    };
    let mut crates = Vec::with_capacity(manifest.crates.len());
    let mut contract_roots = BTreeMap::new();
    let mut resource_views = BTreeMap::new();
    let mut bounds_views = BTreeMap::new();
    let mut outer_views = BTreeMap::new();
    for record in manifest.crates {
        let name = record.name.clone();
        if !relative_directory(&record.contracts) {
            return Err(refuse(
                &name,
                format!(
                    "contract directory `{}` is not relative without traversal",
                    record.contracts
                ),
            ));
        }
        let root = base.join(&record.contracts);
        let root_metadata = fs::symlink_metadata(&root)
            .map_err(|e| refuse(&name, format!("{}: {e}", root.display())))?;
        if !root_metadata.is_dir() {
            return Err(refuse(
                &name,
                format!("{} is not a directory", root.display()),
            ));
        }
        if record.modules.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(refuse(
                &name,
                "owned modules are not sorted and unique".into(),
            ));
        }
        if record
            .dependencies
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(refuse(
                &name,
                "dependencies are not sorted and unique".into(),
            ));
        }
        if let Some(module) = record
            .modules
            .iter()
            .find(|m| !valid_contract_module_path(m))
        {
            return Err(refuse(&name, format!("malformed module path `{module}`")));
        }
        let on_disk = contract_files(&root).map_err(|e| refuse(&name, e))?;
        let listed: BTreeSet<String> = record.modules.iter().cloned().collect();
        if on_disk != listed {
            let unlisted: Vec<_> = on_disk.difference(&listed).collect();
            let absent: Vec<_> = listed.difference(&on_disk).collect();
            return Err(refuse(
                &name,
                format!(
                    "owned modules differ from the contract directory: unlisted {unlisted:?}, absent {absent:?}"
                ),
            ));
        }
        if let Some(view) = &record.resource_view {
            if view.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(refuse(
                    &name,
                    "resource view modules are not sorted and unique".into(),
                ));
            }
            if let Some(module) = view.iter().find(|module| !listed.contains(*module)) {
                return Err(refuse(
                    &name,
                    format!("resource view names `{module}`, which the crate does not own"),
                ));
            }
            resource_views.insert(name.clone(), view.clone());
        }
        // Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the bounds view's shape.
        if let Some(view) = &record.bounds_view {
            if view.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(refuse(
                    &name,
                    "bounds view modules are not sorted and unique".into(),
                ));
            }
            if let Some(module) = view.iter().find(|module| !listed.contains(*module)) {
                return Err(refuse(
                    &name,
                    format!("bounds view names `{module}`, which the crate does not own"),
                ));
            }
            if let Some(module) = view.iter().find(|module| {
                record
                    .resource_view
                    .as_ref()
                    .is_some_and(|resource| resource.contains(module))
            }) {
                return Err(refuse(
                    &name,
                    format!("`{module}` is in both its resource view and its bounds view"),
                ));
            }
            bounds_views.insert(name.clone(), view.clone());
        }
        // Card OUTER-CONTRACT: the outer view's shape, disjoint from both.
        if let Some(view) = &record.outer_view {
            if view.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(refuse(
                    &name,
                    "outer view modules are not sorted and unique".into(),
                ));
            }
            if let Some(module) = view.iter().find(|module| !listed.contains(*module)) {
                return Err(refuse(
                    &name,
                    format!("outer view names `{module}`, which the crate does not own"),
                ));
            }
            let in_other = |module: &String| {
                record
                    .resource_view
                    .as_ref()
                    .is_some_and(|other| other.contains(module))
                    || record
                        .bounds_view
                        .as_ref()
                        .is_some_and(|other| other.contains(module))
            };
            if let Some(module) = view.iter().find(|module| in_other(module)) {
                return Err(refuse(
                    &name,
                    format!("`{module}` is in its outer view and in another view"),
                ));
            }
            outer_views.insert(name.clone(), view.clone());
        }
        let mut modules = BTreeMap::new();
        for module in &record.modules {
            let file = root.join(module);
            let bytes =
                fs::read(&file).map_err(|e| refuse(&name, format!("{}: {e}", file.display())))?;
            if let Some(pin) = record.published.get(module) {
                if !is_sha256_hex(pin) {
                    return Err(refuse(&name, format!("invalid SHA-256 pin for `{module}`")));
                }
                if !matches_pin(&bytes, pin) {
                    return Err(refuse(
                        &name,
                        format!(
                            "published module `{module}` differs from its pinned SHA-256; \
                             regenerate the load-path manifest (`python3 providers/contracts.py sync`)"
                        ),
                    ));
                }
            }
            let text =
                String::from_utf8(bytes).map_err(|e| refuse(&name, format!("{module}: {e}")))?;
            modules.insert(module.clone(), text);
        }
        contract_roots.insert(name.clone(), root);
        crates.push(PublishedContractCrate {
            name,
            namespace: record.namespace,
            version: record.version,
            dependencies: record.dependencies.into_iter().collect(),
            published: record.published.into_keys().collect(),
            modules,
        });
    }
    let Some((shipped, base_path, base_sha256)) = extended else {
        return Ok(LoadPathManifest {
            load_path: ContractLoadPath::new(crates)?,
            contract_roots,
            resource_views,
            bounds_views,
            outer_views,
            extension: None,
        });
    };
    let user_crates: BTreeSet<String> = contract_roots.keys().cloned().collect();
    let shipped_crates: BTreeSet<String> = shipped.contract_roots.keys().cloned().collect();
    if let Some(name) = user_crates.intersection(&shipped_crates).next() {
        return Err(refuse(
            name,
            "a user crate has the name of a crate on the extended load path".into(),
        ));
    }
    let mut overrides = Vec::new();
    for record in manifest.overrides.unwrap_or_default() {
        if !user_crates.contains(&record.user_crate)
            || !shipped_crates.contains(&record.shipped_crate)
            || record.function.trim().is_empty()
            || record.reason.trim().is_empty()
        {
            return Err(refuse(
                &origin,
                format!(
                    "override of `{}` must name one of this manifest's crates, a crate on the \
                     extended load path, a function and a reason",
                    record.function
                ),
            ));
        }
        overrides.push(ContractOverride {
            function: record.function,
            user_crate: record.user_crate,
            shipped_crate: record.shipped_crate,
            reason: record.reason,
        });
    }
    let mut checkers = Vec::new();
    for record in manifest.checkers.unwrap_or_default() {
        let owner = crates
            .iter()
            .find(|krate| krate.name == record.user_crate)
            .filter(|krate| krate.modules.contains_key(&record.module));
        let Some(owner) = owner.filter(|_| {
            !record.functions.is_empty()
                && record.functions.windows(2).all(|pair| pair[0] < pair[1])
        }) else {
            return Err(refuse(
                &origin,
                format!(
                    "checker `{}` must name a module one of this manifest's crates owns and \
                     sorted, unique functions",
                    record.package
                ),
            ));
        };
        if !relative_directory(&record.package) || !base.join(&record.package).is_dir() {
            return Err(refuse(
                &origin,
                format!(
                    "checker package `{}` is not a directory relative to the manifest without \
                     traversal",
                    record.package
                ),
            ));
        }
        let (crate_namespace, crate_version, module_sha256) = (
            owner.namespace.clone(),
            owner.version.clone(),
            sha256_hex(owner.modules[&record.module].as_bytes()),
        );
        checkers.push(CheckerBinding {
            user_crate: record.user_crate,
            crate_namespace,
            crate_version,
            module: record.module,
            module_sha256,
            functions: record.functions,
            package: base.join(&record.package),
            package_spelling: record.package,
        });
    }
    let mut all = shipped.load_path.crates().cloned().collect::<Vec<_>>();
    all.extend(crates);
    let mut roots = shipped.contract_roots;
    roots.extend(contract_roots);
    let mut views = shipped.resource_views;
    views.extend(resource_views);
    let mut all_bounds_views = shipped.bounds_views;
    all_bounds_views.extend(bounds_views);
    let mut all_outer_views = shipped.outer_views;
    all_outer_views.extend(outer_views);
    Ok(LoadPathManifest {
        load_path: ContractLoadPath::new(all)?,
        contract_roots: roots,
        resource_views: views,
        bounds_views: all_bounds_views,
        outer_views: all_outer_views,
        extension: Some(LoadPathExtension {
            base: base_path,
            base_sha256,
            shipped_crates,
            user_crates,
            overrides,
            checkers,
        }),
    })
}
