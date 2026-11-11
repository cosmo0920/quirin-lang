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

use qcore_contract::{
    bounds_contract::load_bounds_contract_sources, bounds_hypotheses, contract_loader, hypothesis,
    hypothesis_bindings,
};
use qcore_schemas::hypothesis::{
    ArtifactReference, BindingManifest, Context, Disposition, EnvironmentDefinition, Evidence,
    FormalBinding, Instance, Warrant,
};
use std::collections::BTreeMap;

const PREMISE: &str = "rk_plugin_callback_closure";
const NULL: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/null_deref.qv");
const CLIENT: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv");
const MODEL: &str = include_str!(
    "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/queue_destructor.qv"
);
fn inputs(text: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("selected-property.qv".into(), text.into())])
}
fn erase_plugin_metadata(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let line = line.trim();
            !(line.starts_with("quant proposition #plugin_")
                || line.starts_with("requires resource_hypothesis(plugin_")
                || line.starts_with(
                    "requires resource_premise_decomposition(rk_plugin_callback_closure,",
                )
                // Dependent symbolic metadata must be erased with its graph atoms.
                || line.starts_with("requires resource_lifecycle_v1(introduce_plugin_")
                || line.starts_with(
                    "requires resource_premise_flow_v1(decompose_rk_plugin_callback_closure,",
                )
                || (line.starts_with("// qcore-premise-binding-manifest-v1 ")
                    && line.contains(" rk_plugin_callback_closure ")))
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}
#[test]
fn all_actual_null_occurrences_preserve_interpreted_clauses_and_semantic_identity() {
    let old = load_bounds_contract_sources(&inputs(&erase_plugin_metadata(NULL))).unwrap();
    let new = load_bounds_contract_sources(&inputs(NULL)).unwrap();
    assert_eq!(
        old.functions.keys().collect::<Vec<_>>(),
        new.functions.keys().collect::<Vec<_>>()
    );
    for (name, before) in &old.functions {
        let after = &new.functions[name];
        assert_eq!(
            (
                &before.parameters,
                &before.result,
                &before.clauses,
                &before.contract_review,
                &before.canonical_sha256
            ),
            (
                &after.parameters,
                &after.result,
                &after.clauses,
                &after.contract_review,
                &after.canonical_sha256
            )
        );
    }
    let graphs = bounds_hypotheses::replay(&inputs(NULL)).unwrap();
    assert_eq!(
        graphs
            .values()
            .filter(|g| g.refinements.contains_key(PREMISE))
            .count(),
        71
    );
    assert!(
        hypothesis::replay(&inputs(NULL)).is_err(),
        "bounds sources remain outside the resource vocabulary"
    );
}
#[test]
fn resource_null_and_model_views_keep_actual_duplicate_spellings_separate() {
    let resource = hypothesis::replay(&inputs(CLIENT)).unwrap();
    let null = bounds_hypotheses::replay(&inputs(NULL)).unwrap();
    for name in ["rd_kafka_toppar_new0", "rd_kafka_toppar_fetch_backoff"] {
        assert!(resource[name].refinements.contains_key(PREMISE));
        assert!(null[name].refinements.contains_key(PREMISE));
    }
    assert!(bounds_hypotheses::check_replay(&inputs(NULL), &resource).is_err());
    assert!(bounds_hypotheses::replay(&inputs(CLIENT)).is_err());
    assert!(bounds_hypotheses::replay_model(&inputs(CLIENT)).is_err());
    assert!(bounds_hypotheses::replay_model(&inputs(NULL)).is_err());
    let original = erase_plugin_metadata(MODEL);
    assert_eq!(
        contract_loader::module_premises(MODEL).unwrap(),
        contract_loader::module_premises(&original).unwrap()
    );
    let graph = bounds_hypotheses::replay_model(&inputs(MODEL)).unwrap();
    assert!(
        graph["rd_kafka_q_destroy_final"]
            .refinements
            .contains_key(PREMISE)
    );
    assert!(
        hypothesis::replay(&inputs(MODEL)).is_err(),
        "declared model is not a C resource ABI"
    );
    assert!(bounds_hypotheses::replay(&inputs(MODEL)).is_err());
}
const FIXTURE: &str = r#"module contract;
extern fn make() -> ptr<mut, u8> contract {
 requires bounds_contract_v1;
 requires nonnull_member(S, next);
 requires resource_premise(rk_plugin_callback_closure, "original callback boundary");
 requires resource_hypothesis(plugin_a, "selected build and symbol bindings remain open");
 requires resource_hypothesis(plugin_b, "registered callback lifetime remains open");
 requires resource_premise_decomposition(rk_plugin_callback_closure, plugin_a, plugin_b);
};"#;
#[test]
fn malformed_bounds_metadata_refuses_instead_of_becoming_a_fact_or_new_premise() {
    assert!(bounds_hypotheses::replay(&inputs(FIXTURE)).is_ok());
    for bad in [
        FIXTURE.replace(" requires resource_premise(rk_plugin_callback_closure, \"original callback boundary\");", ""),
        FIXTURE.replace("plugin_a, plugin_b);","plugin_b, plugin_a);"),
        FIXTURE.replace("resource_premise_decomposition(rk_plugin_callback_closure,", "resource_premise_decomposition(missing,"),
        FIXTURE.replace("requires resource_hypothesis(plugin_a", "ensures normal resource_hypothesis(plugin_a"),
        FIXTURE.replace("requires resource_premise_decomposition", "requires resource_premise_all"),
        FIXTURE.replace("plugin_a, plugin_b);", "plugin_a, absent);"),
        FIXTURE.replace("};", " requires resource_premise_decomposition(rk_plugin_callback_closure, plugin_a, plugin_b);\n};"),
    ] { assert!(bounds_hypotheses::replay(&inputs(&bad)).is_err(),"{bad}"); }
}
fn context() -> Context {
    Context {
        artifact: "actual-source-fixture".into(),
        property: "c.null.deref.v1".into(),
        semantic_profile: "selected".into(),
        scope: "scope".into(),
        bounds: "bounded-trace".into(),
        assumptions: "declared-and-open".into(),
        source_identity: "source".into(),
        build_identity: "build".into(),
        evidence_version: "version".into(),
    }
}
#[test]
fn actual_refinement_refutation_is_separate_and_rejects_each_foreign_context_coordinate() {
    let graphs = bounds_hypotheses::replay(&inputs(FIXTURE)).unwrap();
    let graph = graphs["make"].clone();
    let expected = context();
    let first = &graph.refinements[PREMISE].children[0];
    let evidence = Evidence {
        atom: first.clone(),
        atom_sha256: qcore_schemas::sha256_hex(&serde_json::to_vec(&graph.atoms[first]).unwrap()),
        context: expected.clone(),
        disposition: Disposition::Refuted,
        warrant: Warrant::ProviderProposed,
        reference: "hypothetical complete model trace; C source binding Open".into(),
    };
    let instance = Instance {
        graph: graph.clone(),
        graph_sha256: hypothesis::digest(&graph).unwrap(),
        context: expected.clone(),
        evidence: vec![evidence],
    };
    assert_eq!(
        hypothesis::compose_refinement(&instance, &expected, &graph, PREMISE).unwrap(),
        Disposition::Refuted
    );
    assert_eq!(
        hypothesis::compose(
            &instance,
            &expected,
            &graph,
            &graph.refinements[PREMISE].premise
        )
        .unwrap(),
        Disposition::Open
    );
    for field in 0..9 {
        let mut foreign = expected.clone();
        let value = match field {
            0 => &mut foreign.artifact,
            1 => &mut foreign.property,
            2 => &mut foreign.semantic_profile,
            3 => &mut foreign.scope,
            4 => &mut foreign.bounds,
            5 => &mut foreign.assumptions,
            6 => &mut foreign.source_identity,
            7 => &mut foreign.build_identity,
            _ => &mut foreign.evidence_version,
        };
        value.push_str("-different");
        assert!(hypothesis::compose_refinement(&instance, &foreign, &graph, PREMISE).is_err());
    }
}
#[test]
fn explicit_bounds_bindings_require_actual_source_model_audit_and_complete_domains() {
    let expected_graphs = bounds_hypotheses::replay(&inputs(FIXTURE)).unwrap();
    let graph = &expected_graphs["make"];
    let model_path = "proof/QCore/PluginPremiseDecomposition.lean";
    let audit_path = "proof/AuditPluginPremiseDecomposition.lean";
    let model = include_bytes!("../../../proof/QCore/PluginPremiseDecomposition.lean").to_vec();
    let audit = include_bytes!("../../../proof/AuditPluginPremiseDecomposition.lean").to_vec();
    let artifacts = BTreeMap::from([
        (model_path.to_owned(), model.clone()),
        (audit_path.to_owned(), audit.clone()),
    ]);
    let environment = EnvironmentDefinition {
        id: "independently-held-null-fixture".into(),
        variables: BTreeMap::from([(
            "context".into(),
            "All nine exact property-specific caller-held coordinates; source binding Open".into(),
        )]),
    };
    let atoms = graph.refinements[PREMISE]
        .children
        .iter()
        .map(|id| {
            (
                id.clone(),
                FormalBinding {
                    atom_sha256: qcore_schemas::sha256_hex(
                        &serde_json::to_vec(&graph.atoms[id]).unwrap(),
                    ),
                    proposition: "QCore.PluginPremiseDecomposition.loadedArtifacts".into(),
                    model: ArtifactReference {
                        path: model_path.into(),
                        sha256: qcore_schemas::sha256_hex(&model),
                    },
                    audit: ArtifactReference {
                        path: audit_path.into(),
                        sha256: qcore_schemas::sha256_hex(&audit),
                    },
                    environment: environment.id.clone(),
                    quantifiers: vec![
                        "all actual selected model events, interpreted source Open".into(),
                    ],
                    prerequisites: vec![environment.id.clone()],
                    conclusion: "loadedArtifacts e".into(),
                },
            )
        })
        .collect();
    let manifest = BindingManifest {
        version: 1,
        declaration: "make".into(),
        premise: PREMISE.into(),
        graph_sha256: hypothesis::digest(graph).unwrap(),
        source_binding: "open".into(),
        environment,
        atoms,
    };
    let bytes = serde_json::to_vec(&manifest).unwrap();
    let sha = qcore_schemas::sha256_hex(&bytes);
    let expected = BTreeMap::from([(sha.clone(), manifest)]);
    let presented = BTreeMap::from([(sha.clone(), bytes)]);
    let source = inputs(&format!(
        "// qcore-premise-binding-manifest-v1 make {PREMISE} {sha}\n{FIXTURE}"
    ));
    assert!(
        hypothesis_bindings::resolve_bounds(
            &source,
            &expected_graphs,
            &expected,
            &presented,
            &artifacts
        )
        .is_ok()
    );
    assert!(
        hypothesis_bindings::resolve(&source, &expected_graphs, &expected, &presented, &artifacts)
            .is_err()
    );
    assert!(
        hypothesis_bindings::resolve_bounds(
            &source,
            &expected_graphs,
            &expected,
            &BTreeMap::new(),
            &artifacts
        )
        .is_err()
    );
    let mut missing = artifacts.clone();
    missing.remove(model_path);
    assert!(
        hypothesis_bindings::resolve_bounds(
            &source,
            &expected_graphs,
            &expected,
            &presented,
            &missing
        )
        .is_err()
    );
    let mut altered = artifacts;
    altered.get_mut(audit_path).unwrap().push(0);
    assert!(
        hypothesis_bindings::resolve_bounds(
            &source,
            &expected_graphs,
            &expected,
            &presented,
            &altered
        )
        .is_err()
    );
}
