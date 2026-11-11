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

//! Resolve explicit reference inputs beside an independently held exact graph.
//! No source correspondence, proposition disposition or verdict is produced.
use qcore_schemas::hypothesis::{BindingManifest, Graph};
use serde_json::{Value, json};
use std::{collections::BTreeMap, env, fs};
fn read(path: &str) -> Vec<u8> {
    fs::read(path).unwrap()
}
fn main() {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example premise_bindings_replay");
    let args: Vec<_> = env::args().collect();
    assert_eq!(
        args.len(),
        7,
        "CONTRACTS_SIDECAR EXPECTED_GRAPH EXPECTED_MANIFEST PRESENTED_MANIFEST ARTIFACT_INDEX OUTPUT"
    );
    let sidecar_bytes = read(&args[1]);
    let sidecar: Value = serde_json::from_slice(&sidecar_bytes).unwrap();
    let sources: BTreeMap<String, String> = serde_json::from_value(
        sidecar
            .get("sources")
            .or_else(|| sidecar.get("contracts"))
            .unwrap()
            .clone(),
    )
    .unwrap();
    let expected_graph: Graph = qcore_contract::hypothesis::decode(&read(&args[2])).unwrap();
    let expected_bytes = read(&args[3]);
    let expected_manifest: BindingManifest = serde_json::from_slice(&expected_bytes).unwrap();
    assert_eq!(
        serde_json::to_vec(&expected_manifest).unwrap(),
        expected_bytes
    );
    let expected_sha = qcore_schemas::sha256_hex(&expected_bytes);
    let expected_graphs = qcore_contract::hypothesis::replay(&sources).unwrap();
    assert_eq!(
        expected_graphs[&expected_manifest.declaration],
        expected_graph
    );
    let actual_consulted = &sidecar["contract_dependencies"]["declarations"]
        [&expected_manifest.declaration]["hypothesis_graph"];
    if !actual_consulted.is_null() {
        assert_eq!(actual_consulted, &json!(expected_graph));
    }
    let expected_manifests = BTreeMap::from([(expected_sha.clone(), expected_manifest.clone())]);
    let presented = BTreeMap::from([(expected_sha, read(&args[4]))]);
    let index: BTreeMap<String, String> = serde_json::from_slice(&read(&args[5])).unwrap();
    let artifacts = index
        .iter()
        .map(|(name, path)| (name.clone(), read(path)))
        .collect::<BTreeMap<_, _>>();
    let mut publication = json!({
        "$hypothesis_graphs":BTreeMap::from([(expected_manifest.declaration.clone(),expected_graph)]),
        "$hypothesis_binding_requirements":qcore_contract::hypothesis_bindings::requirements(&sources,&expected_graphs).unwrap(),
    });
    qcore_audit::contract_dependencies::publish_checked_bindings(
        &mut publication,
        &sources,
        &expected_graphs,
        &expected_manifests,
        &presented,
        &artifacts,
    )
    .unwrap();
    let output = json!({"mode":"reference-integrity-only; source-binding-open", "actual_consulted_graph_present":!actual_consulted.is_null(),
        "input_contracts_sha256":qcore_schemas::sha256_hex(&sidecar_bytes),"explicit_artifact_paths":index,
        "explicit_artifact_sha256":artifacts.iter().map(|(k,v)|(k.clone(),qcore_schemas::sha256_hex(v))).collect::<BTreeMap<_,_>>(),
        "publication":publication});
    fs::write(&args[6], serde_json::to_vec_pretty(&output).unwrap()).unwrap();
    println!(
        "{}",
        json!({"exit":0,"graph_sha256":expected_manifest.graph_sha256,"bindings":expected_manifest.atoms.len(),"source_binding":"open"})
    );
}
