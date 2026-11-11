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

//! Bounded explicit-input reference checker for the two actual outer envelopes.
//! Matching bytes/identities yields Open source binding, never premise truth.
use qcore_contract::outer_contract::load_outer_contract_sources;
use qcore_schemas::sha256_hex;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};

fn object_keys(value: &Value, wanted: &[&str]) -> Result<(), String> {
    let actual = value.as_object().ok_or("expected object")?;
    let mut keys: Vec<_> = actual.keys().map(String::as_str).collect();
    let mut expected = wanted.to_vec();
    keys.sort();
    expected.sort();
    if keys != expected {
        return Err("catalog: unknown or missing field".into());
    }
    Ok(())
}
fn string<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value[field]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("catalog: missing {field}"))
}
fn reference(value: &Value, artifacts: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    object_keys(value, &["path", "sha256"])?;
    let path = string(value, "path")?;
    if path.starts_with('/')
        || path.contains('\\')
        || path.split('/').any(|p| matches!(p, "" | "." | ".."))
    {
        return Err("catalog: unsafe reference".into());
    }
    if artifacts.get(path).map(|b| sha256_hex(b)) != Some(string(value, "sha256")?.to_owned()) {
        return Err("catalog: missing or altered explicit artifact".into());
    }
    Ok(())
}
fn check(
    expected_bytes: &[u8],
    presented: &[u8],
    sources: &BTreeMap<String, String>,
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<Value, String> {
    let expected: Value = serde_json::from_slice(expected_bytes).map_err(|e| e.to_string())?;
    let catalog: Value = serde_json::from_slice(presented).map_err(|e| e.to_string())?;
    if serde_json::to_vec(&catalog).map_err(|e| e.to_string())? != presented
        || serde_json::to_vec(&expected).map_err(|e| e.to_string())? != expected_bytes
        || catalog != expected
    {
        return Err("catalog: noncanonical or independently unexpected catalog".into());
    }
    object_keys(
        &catalog,
        &[
            "version",
            "source_binding",
            "environment",
            "source_modules",
            "candidate_inventory",
            "loader_sources",
            "envelopes",
        ],
    )?;
    if catalog["version"] != 1 || catalog["source_binding"] != "open" {
        return Err("catalog: version/source binding".into());
    }
    object_keys(&catalog["environment"], &["id", "variables"])?;
    let environment = string(&catalog["environment"], "id")?;
    if catalog["environment"]["variables"]
        .as_object()
        .is_none_or(|m| {
            m.is_empty()
                || m.iter()
                    .any(|(k, v)| k.is_empty() || v.as_str().is_none_or(str::is_empty))
        })
    {
        return Err("catalog: missing explicit environment".into());
    }
    let source_digests: BTreeMap<_, _> = sources
        .iter()
        .map(|(name, text)| (name.clone(), sha256_hex(text.as_bytes())))
        .collect();
    if serde_json::to_value(&source_digests).map_err(|e| e.to_string())?
        != catalog["source_modules"]
    {
        return Err("catalog: source domain/digest".into());
    }
    reference(&catalog["candidate_inventory"], artifacts)?;
    let candidate_hash = string(&catalog["candidate_inventory"], "sha256")?;
    let loaders = catalog["loader_sources"]
        .as_array()
        .filter(|x| !x.is_empty())
        .ok_or("catalog: loader sources required")?;
    for source in loaders {
        reference(source, artifacts)?;
    }
    let outer = load_outer_contract_sources(sources)?;
    let envelopes = catalog["envelopes"]
        .as_object()
        .ok_or("catalog: envelope object")?;
    if envelopes.keys().cloned().collect::<Vec<_>>()
        != vec![
            "flb_external_plugin".to_owned(),
            "rk_plugin_library".to_owned(),
        ]
        || outer.contracts.len() != 2
    {
        return Err("catalog: exact two-envelope domain".into());
    }
    for (name, entry) in envelopes {
        object_keys(
            entry,
            &["definition", "hosting_loaders", "outer_sha256", "atoms"],
        )?;
        let held = &outer.contracts[name];
        if held.defined_by != string(entry, "definition")?
            || outer.digest(name).as_deref() != Some(string(entry, "outer_sha256")?)
            || serde_json::to_value(&held.loaders).map_err(|e| e.to_string())?
                != entry["hosting_loaders"]
        {
            return Err("catalog: outer definition/host/digest mismatch".into());
        }
        if !held
            .statement
            .contains(&format!("outer-candidates:{candidate_hash}"))
        {
            return Err("catalog: candidate not bound by actual statement".into());
        }
        let atoms = entry["atoms"]
            .as_object()
            .filter(|x| !x.is_empty())
            .ok_or("catalog: atoms required")?;
        for binding in atoms.values() {
            object_keys(
                binding,
                &[
                    "proposition",
                    "model",
                    "audit",
                    "environment",
                    "quantifiers",
                    "prerequisites",
                    "conclusion",
                    "mapping",
                    "source_binding",
                ],
            )?;
            if string(binding, "environment")? != environment
                || binding["prerequisites"] != json!([environment])
                || binding["source_binding"] != "open"
                || !string(binding, "proposition")?.starts_with("QCore.PluginPremiseDecomposition.")
                || binding["quantifiers"].as_array().is_none_or(|q| {
                    q.is_empty() || q.iter().any(|v| v.as_str().is_none_or(str::is_empty))
                })
            {
                return Err("catalog: formal/environment/prerequisite record".into());
            }
            string(binding, "conclusion")?;
            string(binding, "mapping")?;
            reference(&binding["model"], artifacts)?;
            reference(&binding["audit"], artifacts)?;
        }
    }
    Ok(
        json!({"reference_integrity":"checked", "source_binding":"open", "catalog_sha256":sha256_hex(presented), "envelopes":envelopes.keys().collect::<Vec<_>>(), "meaning":"Explicit independently held inputs agree; no source truth, premise support, provider authority or verdict is minted."}),
    )
}
fn main() -> Result<(), String> {
    let argv: Vec<_> = std::env::args().collect();
    if argv.len() != 6 {
        return Err(
            "usage: outer_catalog_replay EXPECTED PRESENTED SOURCE_INDEX ARTIFACT_INDEX OUT".into(),
        );
    }
    let expected = fs::read(&argv[1]).map_err(|e| e.to_string())?;
    let presented = fs::read(&argv[2]).map_err(|e| e.to_string())?;
    let source_index: BTreeMap<String, String> =
        serde_json::from_slice(&fs::read(&argv[3]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let artifact_index: BTreeMap<String, String> =
        serde_json::from_slice(&fs::read(&argv[4]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let sources = source_index
        .into_iter()
        .map(|(key, path)| {
            fs::read_to_string(path)
                .map(|v| (key, v))
                .map_err(|e| e.to_string())
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let artifacts = artifact_index
        .into_iter()
        .map(|(key, path)| fs::read(path).map(|v| (key, v)).map_err(|e| e.to_string()))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let result = check(&expected, &presented, &sources, &artifacts)?;
    fs::write(
        &argv[5],
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
