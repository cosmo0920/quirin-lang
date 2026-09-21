//! Explicit, immutable contract data from an independently pinned inventory.
//! Hash equality identifies declared bytes; it neither authenticates the author
//! nor proves predicates. No provider executable is read or launched here.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

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

fn asset_path(value: &str) -> bool {
    value.starts_with("contracts/")
        && value.ends_with(".qcore")
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
        if sha256_hex(&bytes) != expected_sha256 {
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
            if sha256_hex(&bytes) != hash {
                return Err(format!("contract digest mismatch: {path}"));
            }
            let source = String::from_utf8(bytes).map_err(|e| e.to_string())?;
            if source.trim().is_empty() {
                return Err(format!("empty contract: {path}"));
            }
            contracts.insert(path, source);
        }
        Ok(Self { contracts })
    }

    pub fn contract(&self, path: &str) -> Result<&str, String> {
        self.contracts
            .get(path)
            .map(String::as_str)
            .ok_or_else(|| format!("unselected contract: {path}"))
    }
}
