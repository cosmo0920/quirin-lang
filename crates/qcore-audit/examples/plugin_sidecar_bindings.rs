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

//! Explicit property-scoped replay of this card's source/manifest/publication.
//! INPUT enumerates caller-held source paths and artifacts; no discovery/truth.
use qcore_schemas::hypothesis::{BindingManifest, Graph};
use serde_json::{Value, json};
use std::{collections::BTreeMap, env, fs};
fn read(path: &str) -> Vec<u8> {
    fs::read(path).unwrap()
}
fn object_file(path: &Value) -> BTreeMap<String, String> {
    serde_json::from_slice(&read(path.as_str().unwrap())).unwrap()
}
fn main() {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example plugin_sidecar_bindings");
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 3, "INPUT OUTPUT");
    let spec: Value = serde_json::from_slice(&read(&args[1])).unwrap();
    let mode = spec["mode"].as_str().unwrap();
    let index = object_file(&spec["source_index"]);
    let sources = index
        .iter()
        .map(|(name, path)| (name.clone(), String::from_utf8(read(path)).unwrap()))
        .collect::<BTreeMap<_, _>>();
    let graphs: BTreeMap<String, Graph> =
        serde_json::from_slice(&read(spec["held_graphs"].as_str().unwrap())).unwrap();
    let held_index = object_file(&spec["held_manifest_index"]);
    let held: BTreeMap<String, BindingManifest> = held_index
        .iter()
        .map(|(sha, path)| {
            let raw = read(path);
            assert_eq!(qcore_schemas::sha256_hex(&raw), *sha);
            let value: BindingManifest = serde_json::from_slice(&raw).unwrap();
            assert_eq!(serde_json::to_vec(&value).unwrap(), raw);
            (sha.clone(), value)
        })
        .collect();
    let presented = object_file(&spec["presented_manifest_index"])
        .iter()
        .map(|(sha, path)| (sha.clone(), read(path)))
        .collect();
    let artifacts = object_file(&spec["artifact_index"])
        .iter()
        .map(|(name, path)| (name.clone(), read(path)))
        .collect();
    let requirements =
        qcore_contract::hypothesis_bindings::requirements(&sources, &graphs).unwrap();
    let input = read(spec["publication"].as_str().unwrap());
    let mut matched = 0usize;
    let mut contexts = Vec::new();
    let mut unconsulted = Vec::new();
    let bindings = match mode {
        "resource" => {
            let sidecar: Value = serde_json::from_slice(&input).unwrap();
            let actual = sidecar
                .get("sources")
                .or_else(|| sidecar.get("contracts"))
                .unwrap();
            // Explicitly isolated selected module view, not an opportunistic manifest
            // subset of a full-domain resolver. Every held source must equal the real
            // sidecar text; unrelated cJSON sources remain outside this view deliberately.
            for (name, source) in &sources {
                assert_eq!(
                    actual[name].as_str(),
                    Some(source.as_str()),
                    "held property source differs from actual consulted source"
                );
            }
            let result = qcore_contract::hypothesis_bindings::resolve(
                &sources, &graphs, &held, &presented, &artifacts,
            )
            .unwrap();
            for requirement in &requirements {
                let Some(actual) =
                    sidecar["contract_dependencies"]["declarations"].get(&requirement.declaration)
                else {
                    unconsulted.push(requirement.declaration.clone());
                    continue;
                };
                assert_eq!(
                    actual["hypothesis_graph"],
                    json!(graphs[&requirement.declaration])
                );
                assert_eq!(
                    actual["hypothesis_evidence"]["graph_sha256"],
                    requirement.graph_sha256
                );
                let published = actual["hypothesis_binding_requirements"]
                    .as_array()
                    .unwrap();
                assert!(published.contains(&json!(requirement)));
                for context in actual["hypothesis_evidence"]["contexts"]
                    .as_array()
                    .unwrap()
                {
                    contexts.push(
                        serde_json::from_value::<qcore_schemas::hypothesis::Context>(
                            context.clone(),
                        )
                        .unwrap(),
                    );
                }
                matched += 1;
            }
            result
        }
        "null" => {
            let result = qcore_contract::hypothesis_bindings::resolve_bounds(
                &sources, &graphs, &held, &presented, &artifacts,
            )
            .unwrap();
            let environment =
                qcore_contract::bounds_contract::load_bounds_contract_sources(&sources).unwrap();
            let view = qcore_audit::null_hypotheses::View::new(&environment, &sources).unwrap();
            for line in std::str::from_utf8(&input)
                .unwrap()
                .lines()
                .filter(|line| !line.trim().is_empty())
            {
                let row: Value = serde_json::from_str(line).unwrap();
                let records = row["null"]["premises"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|p| qcore_schemas::hypothesis::ErasedPremise {
                        name: p["name"].as_str().unwrap().into(),
                        statement: p["statement"].as_str().unwrap().into(),
                        origin: "declared".into(),
                    })
                    .collect::<Vec<_>>();
                let mut graph = qcore_contract::hypothesis::legacy_records(&records).unwrap();
                let references = view.attach(&row, &mut graph).unwrap();
                assert_eq!(row["hypothesis_graph"], json!(graph));
                assert_eq!(
                    row["hypothesis_graph_sha256"],
                    qcore_contract::hypothesis::digest(&graph).unwrap()
                );
                if references.is_empty() {
                    assert!(row.get("hypothesis_binding_requirements").is_none());
                } else {
                    assert_eq!(row["hypothesis_binding_requirements"], json!(references));
                    matched += 1;
                }
                if let Some(context) = row.get("hypothesis_context") {
                    let context: qcore_schemas::hypothesis::Context =
                        serde_json::from_value(context.clone()).unwrap();
                    assert_eq!(context.property, qcore_c_gateway::null_deref::NULL_PROPERTY);
                    contexts.push(context);
                }
            }
            result
        }
        _ => panic!("resource or null publication required; model replay is separate"),
    };
    fs::write(&args[2],serde_json::to_vec_pretty(&json!({"mode":mode,"view":"explicit independently held property/module subset","publication_sha256":qcore_schemas::sha256_hex(&input),"requirements":requirements,"checked_bindings":bindings,"matched_publications":matched,"unconsulted_declarations":unconsulted,"published_contexts":contexts,"source_binding":"open","evidence_dispositions_minted":0})).unwrap()).unwrap();
    println!(
        "{}",
        json!({"mode":mode,"matched_publications":matched,"source_binding":"open"})
    );
}
