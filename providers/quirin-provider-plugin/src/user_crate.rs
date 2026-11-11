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

//! A user contract crate, read total-or-refuse.
//!
//! The layout is the `quirin-provider-*` data crates' layout:
//!
//! ```text
//! quirin-provider-acme/
//!   Cargo.toml            [package] name = "quirin-provider-acme", version; [dependencies]
//!   src/lib.rs            pub const NAMESPACE: &str = "acme";  pub const PUBLISHED: &[&str] = &[…];
//!   contracts/acme/*.qv
//!   contracts.lock        written by `quirin-audit pin` after review (pinned contracts)
//!   quirin-plugin.json    checker bindings and explicit overrides (this SDK)
//! ```
//!
//! Only what the load path needs is read, and anything that cannot be read in
//! the one admitted spelling is refused rather than guessed: `Cargo.toml`'s
//! `[package]` `name` and `version` and its `[dependencies]` keys that name a
//! contract crate; `src/lib.rs`'s `NAMESPACE` and `PUBLISHED` constants, in
//! the spelling `providers/contracts.py` reads; every `.qv` or legacy `.qcore` file under
//! `contracts/` (no symbolic link, no other asset).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    CONTRACT_CRATE_PREFIX, PLUGIN_MANIFEST, PLUGIN_MANIFEST_SCHEMA, derive_namespace,
    relative_without_traversal,
};

/// A checker bound to a module's contracts, as `quirin-plugin.json` states
/// it. `package` is relative to the crate directory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckerDeclaration {
    pub module: String,
    pub functions: Vec<String>,
    pub package: String,
}

/// An explicit override of a shipped contract for the same function. The
/// audit refuses a collision without one and reports every one it applies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverrideDeclaration {
    pub function: String,
    pub shipped_crate: String,
    pub reason: String,
}

/// `quirin-plugin.json`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub schema_version: u32,
    #[serde(default)]
    pub checkers: Vec<CheckerDeclaration>,
    #[serde(default)]
    pub overrides: Vec<OverrideDeclaration>,
}

/// A validated user contract crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserCrate {
    pub directory: PathBuf,
    pub name: String,
    pub namespace: String,
    pub version: String,
    /// `[dependencies]` keys naming a contract crate, sorted.
    pub dependencies: Vec<String>,
    /// Owned modules, logical path to text, sorted.
    pub modules: BTreeMap<String, String>,
    pub published: Vec<String>,
    pub manifest: PluginManifest,
}

fn refuse(directory: &Path, message: impl std::fmt::Display) -> String {
    format!("{}: {message}", directory.display())
}

/// The value of `key = "…"` on one line, if it is exactly that.
fn quoted(line: &str, key: &str) -> Option<String> {
    let rest = line
        .strip_prefix(key)?
        .trim_start()
        .strip_prefix('=')?
        .trim();
    let inner = rest.strip_prefix('"')?.strip_suffix('"')?;
    (!inner.contains('"')).then(|| inner.to_owned())
}

/// `[package]` name and version, and the contract-crate keys of
/// `[dependencies]`.
fn cargo_manifest(directory: &Path) -> Result<(String, String, Vec<String>), String> {
    let text = fs::read_to_string(directory.join("Cargo.toml"))
        .map_err(|error| refuse(directory, format!("Cargo.toml: {error}")))?;
    let mut section = String::new();
    let (mut name, mut version) = (None, None);
    let mut dependencies = BTreeSet::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line.to_owned();
            continue;
        }
        if section == "[package]" {
            if let Some(value) = quoted(line, "name") {
                name = Some(value);
            } else if let Some(value) = quoted(line, "version") {
                version = Some(value);
            }
        } else if section == "[dependencies]"
            && let Some((key, _)) = line.split_once('=')
        {
            let key = key.trim();
            if key.starts_with(CONTRACT_CRATE_PREFIX) || key.starts_with("qcore-provider-") {
                dependencies.insert(key.to_owned());
            }
        }
    }
    match (name, version) {
        (Some(name), Some(version)) => Ok((name, version, dependencies.into_iter().collect())),
        _ => Err(refuse(
            directory,
            "Cargo.toml must state [package] name = \"…\" and version = \"…\" on their own lines",
        )),
    }
}

/// `NAMESPACE` and `PUBLISHED`, in the one spelling `providers/contracts.py`
/// reads.
fn constants(directory: &Path) -> Result<(String, Vec<String>), String> {
    let path = directory.join("src/lib.rs");
    let text = fs::read_to_string(&path)
        .map_err(|error| refuse(directory, format!("src/lib.rs: {error}")))?;
    let namespaces: Vec<&str> = text
        .lines()
        .filter_map(|line| {
            line.strip_prefix("pub const NAMESPACE: &str = \"")?
                .strip_suffix("\";")
        })
        .collect();
    let published: Vec<&str> = text
        .lines()
        .filter_map(|line| {
            line.strip_prefix("pub const PUBLISHED: &[&str] = &[")?
                .strip_suffix("];")
        })
        .collect();
    let ([namespace], [published]) = (namespaces.as_slice(), published.as_slice()) else {
        return Err(refuse(
            directory,
            "src/lib.rs must record one `pub const NAMESPACE: &str = \"…\";` and one \
             `pub const PUBLISHED: &[&str] = &[…];` line",
        ));
    };
    let mut modules = Vec::new();
    for item in published
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let module = item
            .strip_prefix('"')
            .and_then(|item| item.strip_suffix('"'))
            .ok_or_else(|| refuse(directory, format!("malformed PUBLISHED entry {item}")))?;
        modules.push(module.to_owned());
    }
    if modules.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(refuse(directory, "PUBLISHED must be sorted and unique"));
    }
    Ok(((*namespace).to_owned(), modules))
}

fn valid_module_path(module: &str) -> bool {
    (module.ends_with(".qcore") || module.ends_with(".qv"))
        && relative_without_traversal(module)
        && module.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_/-.".contains(&byte)
        })
}

/// Every `.qv` or legacy `.qcore` file under `contracts/`, as logical paths.
fn contract_tree(directory: &Path) -> Result<BTreeMap<String, String>, String> {
    let root = directory.join("contracts");
    let metadata = fs::symlink_metadata(&root)
        .map_err(|error| refuse(directory, format!("contracts/: {error}")))?;
    if !metadata.is_dir() {
        return Err(refuse(directory, "contracts/ is not a directory"));
    }
    let mut modules = BTreeMap::new();
    let mut pending = vec![root.clone()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current).map_err(|error| refuse(directory, error))? {
            let entry = entry.map_err(|error| refuse(directory, error))?;
            let kind = entry
                .file_type()
                .map_err(|error| refuse(directory, error))?;
            let path = entry.path();
            if kind.is_dir() {
                pending.push(path);
                continue;
            }
            let logical = path
                .strip_prefix(&root)
                .map_err(|error| refuse(directory, error))?
                .to_string_lossy()
                .replace('\\', "/");
            if !kind.is_file() || !valid_module_path(&logical) {
                return Err(refuse(
                    directory,
                    format!("unexpected contract asset `{logical}`"),
                ));
            }
            let alias = if let Some(stem) = logical.strip_suffix(".qv") {
                format!("{stem}.qcore")
            } else {
                format!("{}.qv", logical.strip_suffix(".qcore").unwrap())
            };
            if modules.contains_key(&alias) {
                return Err(refuse(
                    directory,
                    format!("contract source aliases `{logical}` and `{alias}` share one stem"),
                ));
            }
            let text = fs::read_to_string(&path).map_err(|error| refuse(directory, error))?;
            modules.insert(logical, text);
        }
    }
    if modules.is_empty() {
        return Err(refuse(directory, "contracts/ owns no module"));
    }
    Ok(modules)
}

impl PluginManifest {
    /// Validate against the crate's own modules.
    ///
    /// # Errors
    ///
    /// Any declaration that names no owned module, lists no function or an
    /// unsorted list, or names a package outside the crate.
    pub fn validate(&self, modules: &BTreeMap<String, String>) -> Result<(), String> {
        if self.schema_version != PLUGIN_MANIFEST_SCHEMA {
            return Err(format!(
                "{PLUGIN_MANIFEST} schema {} is not {PLUGIN_MANIFEST_SCHEMA}",
                self.schema_version
            ));
        }
        for checker in &self.checkers {
            if !modules.contains_key(&checker.module) {
                return Err(format!(
                    "checker `{}` names `{}`, which the crate does not own",
                    checker.package, checker.module
                ));
            }
            if checker.functions.is_empty()
                || checker.functions.windows(2).any(|pair| pair[0] >= pair[1])
            {
                return Err(format!(
                    "checker `{}` must list sorted, unique functions",
                    checker.package
                ));
            }
            if !relative_without_traversal(&checker.package) {
                return Err(format!(
                    "checker package `{}` must be relative to the crate without traversal",
                    checker.package
                ));
            }
        }
        for record in &self.overrides {
            if record.function.trim().is_empty()
                || record.reason.trim().is_empty()
                || !record.shipped_crate.starts_with(CONTRACT_CRATE_PREFIX)
                    && !record.shipped_crate.starts_with("qcore-provider-")
            {
                return Err(format!(
                    "override of `{}` must name a function, a shipped contract crate and a reason",
                    record.function
                ));
            }
        }
        Ok(())
    }
}

impl UserCrate {
    /// Read and validate the crate at `directory`.
    ///
    /// # Errors
    ///
    /// Any departure from the layout above.
    pub fn read(directory: &Path) -> Result<Self, String> {
        let (name, version, dependencies) = cargo_manifest(directory)?;
        let derived = derive_namespace(&name).ok_or_else(|| {
            refuse(
                directory,
                format!("a contract crate is named `{CONTRACT_CRATE_PREFIX}NAME`, not `{name}`"),
            )
        })?;
        let (namespace, published) = constants(directory)?;
        if namespace != derived {
            return Err(refuse(
                directory,
                format!("recorded namespace `{namespace}` is not the derived `{derived}`"),
            ));
        }
        let modules = contract_tree(directory)?;
        if let Some(module) = published
            .iter()
            .find(|module| !modules.contains_key(*module))
        {
            return Err(refuse(
                directory,
                format!("publishes `{module}`, which it does not own"),
            ));
        }
        let manifest_path = directory.join(PLUGIN_MANIFEST);
        let manifest = if manifest_path.exists() {
            let bytes = fs::read(&manifest_path).map_err(|error| refuse(directory, error))?;
            serde_json::from_slice::<PluginManifest>(&bytes)
                .map_err(|error| refuse(directory, format!("{PLUGIN_MANIFEST}: {error}")))?
        } else {
            PluginManifest {
                schema_version: PLUGIN_MANIFEST_SCHEMA,
                ..PluginManifest::default()
            }
        };
        manifest
            .validate(&modules)
            .map_err(|error| refuse(directory, error))?;
        Ok(Self {
            directory: directory.to_path_buf(),
            name,
            namespace,
            version,
            dependencies,
            modules,
            published,
            manifest,
        })
    }
}
