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

//! Card-local explicit producer/replayer. Reads only paths enumerated by INPUT.
//! Catalog hashes bind formal references; they grant no source correspondence.
use qcore_schemas::hypothesis::{
    ArtifactReference, BindingManifest, EnvironmentDefinition, FormalBinding, Graph,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, env, fs};
fn read(path: &str) -> Vec<u8> {
    fs::read(path).unwrap()
}
fn replay(mode: &str, sources: &BTreeMap<String, String>) -> BTreeMap<String, Graph> {
    match mode {
        "resource" => qcore_contract::hypothesis::replay(sources),
        "null" => qcore_contract::bounds_hypotheses::replay(sources),
        "model" => qcore_contract::bounds_hypotheses::replay_model(sources),
        _ => panic!("unsupported property view"),
    }
    .unwrap()
}
fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 3, "INPUT OUTPUT");
    let input: Value = serde_json::from_slice(&read(&args[1])).unwrap();
    let mode = input["mode"].as_str().unwrap();
    let mut sources = BTreeMap::new();
    for (key, path) in input["sources"].as_object().unwrap() {
        sources.insert(
            key.clone(),
            String::from_utf8(read(path.as_str().unwrap())).unwrap(),
        );
    }
    if let Some(items) = input["elaborated"].as_array() {
        let load = qcore_contract::load_path_manifest::load_contract_load_path_manifest(
            std::path::Path::new(input["load_path"].as_str().unwrap()),
        )
        .unwrap();
        for item in items {
            let module = qcore_contract::contract_inclusion::elaborate_contract_module(
                &load.load_path,
                item["crate"].as_str().unwrap(),
                item["module"].as_str().unwrap(),
            )
            .unwrap();
            assert!(sources.insert(module.key, module.text).is_none());
        }
    }
    let graphs = replay(mode, &sources);
    let design: Value = serde_json::from_slice(&read(input["design"].as_str().unwrap())).unwrap();
    let mut environment: EnvironmentDefinition =
        serde_json::from_value(design["environment"].clone()).unwrap();
    environment.id = format!("{}_{}_v1", environment.id, mode);
    environment.variables.insert(
        "property_view".into(),
        format!("Exact independently held {mode} source view; no cross-property Context transport"),
    );
    let model_path = "proof/QCore/PluginPremiseDecomposition.lean";
    let audit_path = "proof/AuditPluginPremiseDecomposition.lean";
    let model = read(input["model"].as_str().unwrap());
    let audit = read(input["audit"].as_str().unwrap());
    let model_ref = ArtifactReference {
        path: model_path.into(),
        sha256: qcore_schemas::sha256_hex(&model),
    };
    let audit_ref = ArtifactReference {
        path: audit_path.into(),
        sha256: qcore_schemas::sha256_hex(&audit),
    };
    let premise = "rk_plugin_callback_closure";
    let mut manifests = BTreeMap::new();
    let mut declaration_sha = BTreeMap::new();
    for (name, graph) in &graphs {
        let Some(refinement) = graph.refinements.get(premise) else {
            continue;
        };
        let mut atoms = BTreeMap::new();
        for id in &refinement.children {
            let spec = design["callback_atoms"]
                .as_array()
                .unwrap()
                .iter()
                .find(|spec| spec["id"].as_str() == Some(id.as_str()))
                .unwrap();
            atoms.insert(id.clone(),FormalBinding{atom_sha256:qcore_schemas::sha256_hex(&serde_json::to_vec(&graph.atoms[id]).unwrap()),proposition:spec["proposition"].as_str().unwrap().into(),model:model_ref.clone(),audit:audit_ref.clone(),environment:environment.id.clone(),quantifiers:vec!["For every event of the exact selected bounded complete loader trace e, with artifact, configuration, registration, ownership and lifecycle identities; C/model binding remains Open".into()],prerequisites:vec![environment.id.clone()],conclusion:spec["conclusion"].as_str().unwrap().into()});
        }
        let manifest = BindingManifest {
            version: 1,
            declaration: name.clone(),
            premise: premise.into(),
            graph_sha256: qcore_contract::hypothesis::digest(graph).unwrap(),
            source_binding: "open".into(),
            environment: environment.clone(),
            atoms,
        };
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let sha = qcore_schemas::sha256_hex(&bytes);
        declaration_sha.insert(name.clone(), sha.clone());
        manifests.insert(sha, String::from_utf8(bytes).unwrap());
    }
    // Replay markers only after the caller installs exactly these canonical hashes.
    let mut integrity = None;
    if input["check"].as_bool() == Some(true) {
        let expected_graphs: BTreeMap<String, Graph> =
            serde_json::from_slice(&read(input["held_graphs"].as_str().unwrap())).unwrap();
        assert_eq!(
            graphs, expected_graphs,
            "independently held property graph replay"
        );
        let held_index: BTreeMap<String, String> =
            serde_json::from_slice(&read(input["held_manifest_index"].as_str().unwrap())).unwrap();
        let presented_index: BTreeMap<String, String> =
            serde_json::from_slice(&read(input["presented_manifest_index"].as_str().unwrap()))
                .unwrap();
        let held: BTreeMap<String, BindingManifest> = held_index
            .iter()
            .map(|(sha, path)| {
                let bytes = read(path);
                assert_eq!(qcore_schemas::sha256_hex(&bytes), *sha);
                let manifest: BindingManifest = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(serde_json::to_vec(&manifest).unwrap(), bytes);
                (sha.clone(), manifest)
            })
            .collect();
        let presented = presented_index
            .iter()
            .map(|(sha, path)| (sha.clone(), read(path)))
            .collect();
        let artifact_index: BTreeMap<String, String> =
            serde_json::from_slice(&read(input["artifact_index"].as_str().unwrap())).unwrap();
        let artifacts = artifact_index
            .iter()
            .map(|(key, path)| (key.clone(), read(path)))
            .collect();
        let result = match mode {
            "resource" => qcore_contract::hypothesis_bindings::resolve(
                &sources, &graphs, &held, &presented, &artifacts,
            ),
            "null" => qcore_contract::hypothesis_bindings::resolve_bounds(
                &sources, &graphs, &held, &presented, &artifacts,
            ),
            "model" => qcore_contract::hypothesis_bindings::resolve_model(
                &sources, &graphs, &held, &presented, &artifacts,
            ),
            _ => unreachable!(),
        }
        .unwrap();
        integrity = Some(result);
    }
    fs::write(&args[2],serde_json::to_vec_pretty(&json!({"mode":mode,"sources":sources,"graphs":graphs,"canonical_manifests":manifests,"declaration_manifest_sha256":declaration_sha,"reference_integrity":integrity,"source_binding":"open"})).unwrap()).unwrap();
    println!(
        "{}",
        json!({"mode":mode,"graphs":graphs.len(),"selected_refinements":declaration_sha.len(),"checked":input["check"]})
    );
}
