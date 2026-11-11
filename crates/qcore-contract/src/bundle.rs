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

//! Explicit, immutable contract data from an independently pinned inventory.
//! Hash equality identifies declared bytes; it neither authenticates the author
//! nor proves predicates. No provider executable is read or launched here.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use qcore_frontend::contract_inclusion::{
    contains_contract_inclusion_syntax, scan_contract_inclusion, valid_contract_crate_name,
    valid_contract_module_path,
};
use qcore_schemas::{PluginId, sha256_hex};
use serde::Deserialize;

const MAX_MANIFEST: u64 = 1_048_576;
const MAX_FILE: u64 = 8_388_608;
const MAX_TOTAL: u64 = 32 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    schema_version: u32,
    plugin_id: String,
    // The custom visitor rejects duplicate keys rather than overwriting them.
    #[serde(deserialize_with = "distinct_contracts")]
    contracts: BTreeMap<String, String>,
    /// Card CONTRACT-INHERIT-ADOPTION: the pinned identity of every module a
    /// packaged contract reaches through `#include "crate:module";`, keyed by
    /// that target. Absent for a package that includes nothing.
    #[serde(default)]
    includes: Option<BTreeMap<String, IncludePin>>,
}

/// One pinned included parent: crate, crate version, module path and the
/// SHA-256 of the text installed at `includes/<crate>/<module>`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IncludePin {
    #[serde(rename = "crate")]
    crate_name: String,
    version: String,
    module: String,
    sha256: String,
}

fn distinct_contracts<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a distinct contract path to SHA-256 map")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            let mut result = BTreeMap::new();
            while let Some((path, digest)) = map.next_entry::<String, String>()? {
                if result.len() >= 4096 || result.insert(path, digest).is_some() {
                    return Err(serde::de::Error::custom(
                        "duplicate or excessive contract inventory",
                    ));
                }
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Visitor)
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn matches_checkout_digest(bytes: &[u8], expected: &str) -> bool {
    if sha256_hex(bytes) == expected {
        return true;
    }
    if !cfg!(windows) {
        return false;
    }
    // Git's Windows checkout may translate pinned LF text to CRLF. Accept
    // only bytes that recover the exact pinned content after that translation.
    let normalized = bytes
        .split(|byte| *byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .collect::<Vec<_>>();
    sha256_hex(&normalized.join(&b'\n')) == expected
}

fn asset_path(value: &str) -> bool {
    value.starts_with("contracts/")
        && (value.ends_with(".qcore") || value.ends_with(".qv"))
        && !value.contains(['\\', ':'])
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn read(path: &Path, maximum: u64) -> Result<Vec<u8>, String> {
    if !fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err(format!(
            "{} is not a regular, non-symlink file",
            path.display()
        ));
    }
    let file = File::open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("opened asset is not a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > maximum {
        return Err("contract bundle byte budget exceeded".into());
    }
    Ok(bytes)
}

fn files(
    root: &Path,
    current: &Path,
    result: &mut Vec<PathBuf>,
    entries: &mut usize,
    depth: usize,
) -> Result<(), String> {
    if depth > 32 {
        return Err("contract directory nesting exceeds 32".into());
    }
    if !fs::symlink_metadata(current)
        .map_err(|e| e.to_string())?
        .is_dir()
    {
        return Err("contract root must be a non-symlink directory".into());
    }
    for entry in fs::read_dir(current).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        *entries += 1;
        if *entries > 8192 {
            return Err("excessive contract tree".into());
        }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            files(root, &entry.path(), result, entries, depth + 1)?;
        } else if kind.is_file() {
            result.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_path_buf(),
            );
        } else {
            return Err("symlink or special asset in contract tree".into());
        }
    }
    Ok(())
}

/// A snapshot of all declared files in the existing provider `contracts.json`
/// format. Construction requires a caller-held manifest digest and owner ID.
/// Protect the selected directory from concurrent mutation during loading;
/// these filesystem checks are not an OS confinement or attestation mechanism.
#[derive(Debug)]
pub struct ContractBundle {
    contracts: BTreeMap<String, String>,
    included: BTreeMap<String, String>,
}

/// The `crate:module` targets a text includes; a malformed directive refuses.
fn include_targets(name: &str, source: &str) -> Result<Vec<String>, String> {
    if !contains_contract_inclusion_syntax(source) {
        return Ok(Vec::new());
    }
    let syntax = scan_contract_inclusion(source).map_err(|e| format!("{name}: {e}"))?;
    Ok(syntax.includes.iter().map(|d| d.target()).collect())
}

/// Card CONTRACT-INHERIT-ADOPTION: the Rust twin of `providers/package.py
/// verify`'s include rule. A bundle whose contracts use `#include` must pin
/// every module they reach (transitively) and carry exactly those texts under
/// `includes/<crate>/<module>`; an unpinned directive, a changed, missing or
/// stray text and an unused pin all refuse. A bundle without pins may carry
/// neither a directive nor an `includes/` tree. Returns the pinned texts.
fn load_includes(
    root: &Path,
    pins: Option<BTreeMap<String, IncludePin>>,
    contracts: &BTreeMap<String, String>,
    retained: &mut u64,
) -> Result<BTreeMap<String, String>, String> {
    let tree = root.join("includes");
    let tree_present = fs::symlink_metadata(&tree).is_ok();
    let mut pending = Vec::new();
    for (name, source) in contracts {
        pending.extend(include_targets(name, source)?);
    }
    let Some(pins) = pins else {
        if !pending.is_empty() {
            return Err(format!("contract includes are not pinned: {pending:?}"));
        }
        if tree_present {
            return Err("installed includes/ tree has no pinned identities".into());
        }
        return Ok(BTreeMap::new());
    };
    if pins.is_empty() || pins.len() > 4096 {
        return Err("contract include pins must be a nonempty, bounded object".into());
    }
    crate::extension_compatibility::reject_alias_collisions(pins.keys().map(String::as_str))?;
    let mut expected = Vec::new();
    for (key, pin) in &pins {
        if !valid_contract_crate_name(&pin.crate_name)
            || !valid_contract_module_path(&pin.module)
            || format!("{}:{}", pin.crate_name, pin.module) != *key
        {
            return Err(format!(
                "include pin {key} does not name its crate and module"
            ));
        }
        if pin.version.is_empty() || !digest(&pin.sha256) {
            return Err(format!(
                "include pin {key} has no version or an invalid SHA-256"
            ));
        }
        expected.push(PathBuf::from(format!(
            "includes/{}/{}",
            pin.crate_name, pin.module
        )));
    }
    if !tree_present {
        return Err("pinned included parents are not installed".into());
    }
    let mut actual = Vec::new();
    files(root, &tree, &mut actual, &mut 0, 0)?;
    actual.sort();
    expected.sort();
    if actual != expected {
        return Err("included parent file set differs from the pins".into());
    }
    let mut texts = BTreeMap::new();
    for (key, pin) in &pins {
        let path = root.join(format!("includes/{}/{}", pin.crate_name, pin.module));
        let bytes = read(&path, MAX_FILE.min(MAX_TOTAL - *retained))?;
        *retained += bytes.len() as u64;
        if !matches_checkout_digest(&bytes, &pin.sha256) {
            return Err(format!("included parent changed: {key}"));
        }
        let source = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        crate::extension_compatibility::validate_source_name(key, &source)
            .map_err(|error| format!("{key}: {error}"))?;
        texts.insert(key.clone(), source);
    }
    let mut reached = std::collections::BTreeSet::new();
    while let Some(key) = pending.pop() {
        if !reached.insert(key.clone()) {
            continue;
        }
        let text = texts
            .get(&key)
            .ok_or_else(|| format!("included parent {key} is not pinned"))?;
        pending.extend(include_targets(&key, text)?);
    }
    if reached.len() != pins.len() {
        let unused: Vec<_> = pins.keys().filter(|key| !reached.contains(*key)).collect();
        return Err(format!(
            "pinned included parents no contract reaches: {unused:?}"
        ));
    }
    Ok(texts)
}

impl ContractBundle {
    pub fn load(
        manifest: &Path,
        expected_sha256: &str,
        expected_owner: &str,
    ) -> Result<Self, String> {
        if !digest(expected_sha256) {
            return Err("invalid expected inventory SHA-256".into());
        }
        let bytes = read(manifest, MAX_MANIFEST)?;
        if !matches_checkout_digest(&bytes, expected_sha256) {
            return Err("contract inventory digest mismatch".into());
        }
        let inventory: Inventory = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        PluginId::new(inventory.plugin_id.clone()).map_err(|e| e.to_string())?;
        if inventory.schema_version != 1
            || inventory.plugin_id != expected_owner
            || inventory.contracts.is_empty()
        {
            return Err("unsupported inventory version, owner mismatch, or empty inventory".into());
        }
        if inventory
            .contracts
            .iter()
            .any(|(path, hash)| !asset_path(path) || !digest(hash))
        {
            return Err("invalid contract path or digest".into());
        }
        crate::extension_compatibility::reject_alias_collisions(
            inventory.contracts.keys().map(String::as_str),
        )?;
        let root = manifest
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let mut actual = Vec::new();
        files(&root, &root.join("contracts"), &mut actual, &mut 0, 0)?;
        actual.sort();
        let mut expected = inventory
            .contracts
            .keys()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        expected.sort();
        if actual != expected {
            return Err("contract file set differs from inventory".into());
        }
        let mut contracts = BTreeMap::new();
        let mut retained = 0;
        for (path, hash) in inventory.contracts {
            let bytes = read(&root.join(&path), MAX_FILE.min(MAX_TOTAL - retained))?;
            retained += bytes.len() as u64;
            if !matches_checkout_digest(&bytes, &hash) {
                return Err(format!("contract digest mismatch: {path}"));
            }
            let source = String::from_utf8(bytes).map_err(|e| e.to_string())?;
            if source.trim().is_empty() {
                return Err(format!("empty contract: {path}"));
            }
            crate::extension_compatibility::validate_source_name(&path, &source)
                .map_err(|error| format!("{path}: {error}"))?;
            contracts.insert(path, source);
        }
        let included = load_includes(&root, inventory.includes, &contracts, &mut retained)?;
        Ok(Self {
            contracts,
            included,
        })
    }

    /// The pinned text of an included parent, by its `crate:module` key (card
    /// CONTRACT-INHERIT-ADOPTION).
    pub fn included(&self, key: &str) -> Result<&str, String> {
        self.included
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| format!("unpinned included module: {key}"))
    }

    pub fn contract(&self, path: &str) -> Result<&str, String> {
        self.contracts
            .get(path)
            .map(String::as_str)
            .ok_or_else(|| format!("unselected contract: {path}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkout_translation_requires_the_exact_pinned_content() {
        let pinned = sha256_hex(b"first\nsecond\n");
        assert!(matches_checkout_digest(b"first\nsecond\n", &pinned));
        assert_eq!(
            matches_checkout_digest(b"first\r\nsecond\r\n", &pinned),
            cfg!(windows)
        );
        assert!(!matches_checkout_digest(b"first\rchanged\n", &pinned));
    }
}
