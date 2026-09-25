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
//!       "modules": ["libc/errno.qcore", "libc/memory.qcore"],
//!       "published": { "libc/memory.qcore": "<sha256>" },
//!       "resource_view": ["libc/errno.qcore", "libc/memory.qcore"]
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

/// Every `.qcore` file under `root`, as logical paths. Refuses a symbolic
/// link and any other non-regular entry, so nothing outside the tree is read.
fn qcore_files(root: &Path) -> Result<BTreeSet<String>, String> {
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
                if path.extension().is_some_and(|ext| ext == "qcore") {
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
    Ok(found)
}

/// Read and validate a load-path manifest (see the module documentation).
/// Relative `contracts` directories resolve against the manifest's own
/// directory.
pub fn load_contract_load_path_manifest(
    path: &Path,
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
    let mut crates = Vec::with_capacity(manifest.crates.len());
    let mut contract_roots = BTreeMap::new();
    let mut resource_views = BTreeMap::new();
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
        let on_disk = qcore_files(&root).map_err(|e| refuse(&name, e))?;
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
    Ok(LoadPathManifest {
        load_path: ContractLoadPath::new(crates)?,
        contract_roots,
        resource_views,
    })
}
