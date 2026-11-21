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

//! Card **SP2-SYMBOLIC-LIBRDKAFKA-PREMISES**: refresh the explicit open binding
//! references after a premise's string statement became a formula (C04.18.9).
//!
//! A named premise erases to a legacy atom whose statement is the premise's
//! statement, so a prose-to-formula rewrite changes the declaration's graph
//! digest and nothing else a binding manifest pins. This tool admits exactly
//! that change: per pinned declaration, the old and new graphs have the same
//! atom and premise IDs, refinements, requirements and version; every
//! non-legacy atom is byte-identical; each changed legacy statement was a
//! prose string and is now the canonical printing of a well-formed formula,
//! the same in the atom and in the premise's erasure. A manifest is then
//! rewritten with the new graph digest as its only change, its marker patched,
//! and every new reference resolved from the explicit inputs. Old manifests
//! and artifacts must be independently supplied; nothing is proved and no
//! binding becomes more than `open`.
//!
//! ```text
//! premise_formula_binding_refresh BEFORE_ROOT AFTER_ROOT OLD_MANIFEST_ROOT OUTPUT_ROOT FILE...
//! ```
use qcore_contract::contract_inclusion::{ContractLoadPath, elaborate_contract_module};
use qcore_contract::{bounds_hypotheses, hypothesis};
use qcore_schemas::hypothesis::{BindingManifest, Graph, Proposition};
use serde_json::json;
use std::{collections::BTreeMap, env, fs, path::Path};

fn materialized(path: &str, text: &str, load_path: &ContractLoadPath) -> Result<String, String> {
    if !qcore_frontend::contract_inclusion::contains_contract_inclusion_syntax(text) {
        return Ok(text.into());
    }
    let (prefix, module) = path
        .split_once("/contracts/")
        .ok_or("provider contract path required")?;
    let owner = prefix.rsplit('/').next().ok_or("provider crate required")?;
    elaborate_contract_module(load_path, owner, module)
        .map(|m| m.text)
        .map_err(|e| e.to_string())
}

fn graphs(path: &str, text: &str) -> Result<BTreeMap<String, Graph>, String> {
    let sources = BTreeMap::from([(path.into(), text.into())]);
    if text.contains("requires bounds_contract_v1;") {
        bounds_hypotheses::replay(&sources)
    } else if text.contains("requires resource_contract_v1;") {
        hypothesis::replay(&sources)
    } else {
        bounds_hypotheses::replay_model(&sources)
    }
}

fn read(path: impl AsRef<Path>) -> Result<Vec<u8>, String> {
    fs::read(path.as_ref()).map_err(|e| format!("{}: {e}", path.as_ref().display()))
}

fn opaque(graph: &Graph, id: &str) -> Option<String> {
    match graph.atoms.get(id).map(|atom| &atom.proposition) {
        Some(Proposition::Opaque { statement }) => Some(statement.clone()),
        _ => None,
    }
}

/// The only admitted difference: legacy statements rewritten from prose to a
/// canonical formula. Returns how many legacy statements changed.
fn statement_rewrite_only(old: &Graph, new: &Graph) -> Result<usize, String> {
    if old.version != new.version
        || old.refinements != new.refinements
        || old.requirements != new.requirements
        || old.atoms.keys().ne(new.atoms.keys())
        || old.premises.keys().ne(new.premises.keys())
    {
        return Err("the graph changed beyond its legacy statements".into());
    }
    let mut changed = 0;
    for (id, premise) in &old.premises {
        let after = &new.premises[id];
        if premise.children != after.children
            || premise.erasure.name != after.erasure.name
            || premise.erasure.origin != after.erasure.origin
        {
            return Err(format!("premise {id} changed beyond its statement"));
        }
        if premise.erasure.statement == after.erasure.statement {
            continue;
        }
        if !id.starts_with("legacy_premise_") {
            return Err(format!("non-legacy premise {id} changed"));
        }
        let atom = id.replacen("legacy_premise_", "legacy_atom_", 1);
        let (Some(before), Some(now)) = (opaque(old, &atom), opaque(new, &atom)) else {
            return Err(format!("{atom}: not an opaque legacy atom"));
        };
        let formula = qcore_frontend::premise_formula::read_source(&now)
            .map_err(|e| format!("{atom}: {e}"))?;
        if before != premise.erasure.statement
            || now != after.erasure.statement
            || formula.canonical != now
            || qcore_frontend::premise_formula::read_source(&before).is_ok()
        {
            return Err(format!("{atom}: not a prose-to-formula rewrite"));
        }
        changed += 1;
    }
    for (id, atom) in &old.atoms {
        let legacy_changed = id.starts_with("legacy_atom_")
            && new.premises[&id.replacen("legacy_atom_", "legacy_premise_", 1)]
                .erasure
                .statement
                != old.premises[&id.replacen("legacy_atom_", "legacy_premise_", 1)]
                    .erasure
                    .statement;
        if !legacy_changed && new.atoms[id] != *atom {
            return Err(format!("atom {id} changed"));
        }
    }
    Ok(changed)
}

fn main() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 5 {
        return Err("BEFORE_ROOT AFTER_ROOT OLD_MANIFEST_ROOT OUTPUT_ROOT FILE...".into());
    }
    let before_root = Path::new(&args[0]);
    let after_root = Path::new(&args[1]);
    let old_root = Path::new(&args[2]);
    let output = Path::new(&args[3]);
    if output.exists() {
        return Err("fresh output directory required".into());
    }
    let current_path = qcore_contract::load_path_manifest::load_contract_load_path_manifest(
        &after_root.join("providers/contract-load-path.json"),
    )
    .map_err(|e| e.to_string())?
    .load_path;
    let mut old_crates = current_path.crates().cloned().collect::<Vec<_>>();
    for file in &args[4..] {
        let (prefix, module) = file
            .split_once("/contracts/")
            .ok_or("provider contract path required")?;
        let owner = prefix.rsplit('/').next().ok_or("provider crate required")?;
        let old = String::from_utf8(read(before_root.join(file))?).map_err(|e| e.to_string())?;
        old_crates
            .iter_mut()
            .find(|c| c.name == owner)
            .ok_or("missing owner")?
            .modules
            .insert(module.into(), old);
    }
    let previous_path = ContractLoadPath::new(old_crates).map_err(|e| e.to_string())?;
    let mut artifacts = BTreeMap::new();
    let mut records = Vec::new();
    let mut manifests = BTreeMap::new();
    let mut patches = Vec::new();
    let mut statements = 0;
    let mut dumped = Vec::new();
    for file in &args[4..] {
        let before = String::from_utf8(read(before_root.join(file))?).map_err(|e| e.to_string())?;
        let after = String::from_utf8(read(after_root.join(file))?).map_err(|e| e.to_string())?;
        let before_view = materialized(file, &before, &previous_path)?;
        let after_view = materialized(file, &after, &current_path)?;
        let old_graphs = graphs(file, &before_view)?;
        let new_graphs = graphs(file, &after_view)?;
        let old_requirements = qcore_contract::hypothesis_bindings::requirements(
            &BTreeMap::from([(file.clone(), before.clone())]),
            &old_graphs,
        )?;
        let mut patched = after.clone();
        for required in old_requirements {
            let bytes = read(old_root.join(format!("{}.json", required.manifest_sha256)))?;
            if qcore_schemas::sha256_hex(&bytes) != required.manifest_sha256 {
                return Err("old manifest hash mismatch".into());
            }
            let mut manifest: BindingManifest =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if manifest.source_binding != "open"
                || manifest.graph_sha256 != required.graph_sha256
                || manifest.declaration != required.declaration
                || manifest.premise != required.premise
            {
                return Err("unexpected old manifest".into());
            }
            let old = &old_graphs[&required.declaration];
            let new = &new_graphs[&required.declaration];
            statements += statement_rewrite_only(old, new)
                .map_err(|e| format!("{file}#{}: {e}", required.declaration))?;
            for (id, binding) in &manifest.atoms {
                let atom = &new.atoms[id];
                if binding.atom_sha256
                    != qcore_schemas::sha256_hex(
                        &serde_json::to_vec(atom).map_err(|e| e.to_string())?,
                    )
                {
                    return Err(format!("{}: bound atom {id} changed", required.declaration));
                }
                for reference in [&binding.model, &binding.audit] {
                    let bytes = read(after_root.join(&reference.path))?;
                    if qcore_schemas::sha256_hex(&bytes) != reference.sha256 {
                        return Err("model/audit identity changed".into());
                    }
                    artifacts.insert(reference.path.clone(), bytes);
                }
            }
            manifest.graph_sha256 = hypothesis::digest(new)?;
            // The old manifests are written in two encodings (compact, or the
            // pretty report file); the refreshed one keeps the old encoding.
            let pretty = bytes.contains(&b'\n');
            let mut bytes = if pretty {
                serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?
            } else {
                serde_json::to_vec(&manifest).map_err(|e| e.to_string())?
            };
            if pretty {
                bytes.push(b'\n');
            }
            let sha = qcore_schemas::sha256_hex(&bytes);
            let needle = format!(
                "// qcore-premise-binding-manifest-v1 {} {} {}",
                required.declaration, required.premise, required.manifest_sha256
            );
            let replacement = format!(
                "// qcore-premise-binding-manifest-v1 {} {} {}",
                required.declaration, required.premise, sha
            );
            if patched.matches(&needle).count() != 1 {
                return Err("ambiguous old marker".into());
            }
            patched = patched.replace(&needle, &replacement);
            records.push(json!({"file":file,"declaration":required.declaration,"premise":required.premise,"previous_manifest_sha256":required.manifest_sha256,"manifest_sha256":sha,"previous_graph_sha256":required.graph_sha256,"graph_sha256":manifest.graph_sha256,"source_binding":"open"}));
            manifests.insert(sha, (manifest, bytes));
        }
        let mut patched_view = after_view.clone();
        for record in records
            .iter()
            .filter(|r| r["file"].as_str() == Some(file.as_str()))
        {
            patched_view = patched_view.replace(
                record["previous_manifest_sha256"].as_str().unwrap(),
                record["manifest_sha256"].as_str().unwrap(),
            );
        }
        let source = BTreeMap::from([(file.clone(), patched_view)]);
        let expected = qcore_contract::hypothesis_bindings::requirements(&source, &new_graphs)?
            .iter()
            .map(|r| r.manifest_sha256.clone())
            .collect::<Vec<_>>();
        let held = expected
            .iter()
            .map(|s| (s.clone(), manifests[s].0.clone()))
            .collect();
        let presented = expected
            .iter()
            .map(|s| (s.clone(), manifests[s].1.clone()))
            .collect();
        if patched.contains("requires bounds_contract_v1;") {
            qcore_contract::hypothesis_bindings::resolve_bounds(
                &source,
                &new_graphs,
                &held,
                &presented,
                &artifacts,
            )?;
        } else if patched.contains("requires resource_contract_v1;") {
            qcore_contract::hypothesis_bindings::resolve(
                &source,
                &new_graphs,
                &held,
                &presented,
                &artifacts,
            )?;
        } else {
            qcore_contract::hypothesis_bindings::resolve_model(
                &source,
                &new_graphs,
                &held,
                &presented,
                &artifacts,
            )?;
        }
        patches.push((after_root.join(file), patched));
        // A frozen expected graph some consumer holds independently (the
        // cJSON detach declaration's), in the graph's canonical encoding.
        if let Ok(declaration) = env::var("QCORE_DUMP_GRAPH")
            && let Some(graph) = new_graphs.get(&declaration)
        {
            let bytes = serde_json::to_vec(graph).map_err(|e| e.to_string())?;
            if hypothesis::decode(&bytes)? != *graph {
                return Err("noncanonical graph encoding".into());
            }
            dumped.push((format!("{declaration}.graph.json"), bytes));
        }
    }
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    for (name, bytes) in &dumped {
        fs::write(output.join(name), bytes).map_err(|e| e.to_string())?;
    }
    for (sha, (_, bytes)) in &manifests {
        fs::write(output.join(format!("{sha}.json")), bytes).map_err(|e| e.to_string())?;
    }
    fs::write(
        output.join("index.json"),
        serde_json::to_vec_pretty(&json!({"source_binding":"open","occurrences":records}))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    for (path, text) in patches {
        fs::write(path, text).map_err(|e| e.to_string())?;
    }
    println!(
        "{} open binding references refreshed; {statements} legacy statements rewritten",
        records.len()
    );
    Ok(())
}
