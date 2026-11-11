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

//! Real selected declaration replay, dependency routing and noninterference.
use qcore_contract::{bounds_contract, bounds_hypotheses, hypothesis};
use qcore_schemas::hypothesis::{ErasedPremise, Graph};
use serde_json::json;
use std::collections::BTreeMap;
const SOURCE: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/null_deref.qv");
const PREMISE: &str = "rk_plugin_callback_closure";
fn actual() -> (qcore_audit::null_hypotheses::View, Graph, serde_json::Value) {
    let sources = BTreeMap::from([("actual-null.qv".into(), SOURCE.into())]);
    let environment = bounds_contract::load_bounds_contract_sources(&sources).unwrap();
    let graphs = bounds_hypotheses::replay(&sources).unwrap();
    let source = &graphs["rd_kafka_topic_new0"];
    let target = &source.premises[&source.refinements[PREMISE].premise];
    // The row already carries another premise, so its legacy index differs from
    // the declaration index. Exact erasure identity, never index/name alone, binds it.
    let mut graph = Graph {
        version: 1,
        atoms: BTreeMap::new(),
        premises: BTreeMap::new(),
        refinements: BTreeMap::new(),
        requirements: Vec::new(),
    };
    hypothesis::append_legacy(
        &mut graph,
        ErasedPremise {
            name: "construction".into(),
            statement: "record construction phase".into(),
            origin: "declared".into(),
        },
    )
    .unwrap();
    hypothesis::append_legacy(&mut graph, target.erasure.clone()).unwrap();
    let row = json!({"artifact":"caller","null":{"property":"c.null.deref.v1","state":"Safe","contract_dependencies":["member:rd_kafka_topic_s.rkt_topic"],"premises":[{"name":target.erasure.name,"statement":target.erasure.statement,"through":["member:rd_kafka_topic_s.rkt_topic"]}]}});
    (
        qcore_audit::null_hypotheses::View::new(&environment, &sources).unwrap(),
        graph,
        row,
    )
}
#[test]
fn actual_member_consultation_retargets_only_metadata_and_keeps_original_nodes_and_null_payload() {
    let (view, mut graph, row) = actual();
    let original = graph.clone();
    let before = row.clone();
    let requirements = view.attach(&row, &mut graph).unwrap();
    assert_eq!(row, before);
    assert_eq!(graph.premises, original.premises);
    for (id, atom) in &original.atoms {
        assert_eq!(graph.atoms.get(id), Some(atom));
    }
    assert_eq!(graph.atoms.len(), original.atoms.len() + 7);
    assert_eq!(graph.refinements.len(), 1);
    assert_eq!(
        graph.premises[&graph.refinements[PREMISE].premise]
            .erasure
            .name,
        PREMISE
    );
    assert!(
        requirements
            .iter()
            .all(|r| r.declaration == "rd_kafka_topic_new0")
    );
    let mut absent = original.clone();
    let mut unrelated = row.clone();
    unrelated["null"]["premises"][0]["through"] = json!(["member:unrelated.field"]);
    unrelated["null"]["contract_dependencies"] = json!(["member:unrelated.field"]);
    assert!(view.attach(&unrelated, &mut absent).unwrap().is_empty());
    assert_eq!(absent, original);
}
#[test]
fn missing_dependency_changed_statement_and_wrong_actual_source_refuse() {
    let (view, original, row) = actual();
    for property in [serde_json::Value::Null, json!("c.resource.custody.v1")] {
        let mut foreign = row.clone();
        foreign["null"]["property"] = property;
        let mut untouched = original.clone();
        assert!(view.attach(&foreign, &mut untouched).is_err());
        assert_eq!(untouched, original);
    }
    let mut missing = row.clone();
    missing["null"]["contract_dependencies"] = json!([]);
    assert!(view.attach(&missing, &mut original.clone()).is_err());
    let mut late_bad = row.clone();
    let mut bad_record = late_bad["null"]["premises"][0].clone();
    bad_record["statement"] = json!("inconsistent second record");
    late_bad["null"]["premises"]
        .as_array_mut()
        .unwrap()
        .push(bad_record);
    let mut unchanged = original.clone();
    assert!(view.attach(&late_bad, &mut unchanged).is_err());
    assert_eq!(unchanged, original, "failed attachment is transactional");
    let mut changed = row;
    changed["null"]["premises"][0]["statement"] = json!("invented neutrality");
    assert!(view.attach(&changed, &mut original.clone()).is_err());
    let sources = BTreeMap::from([("actual-null.qv".into(), SOURCE.into())]);
    let environment = bounds_contract::load_bounds_contract_sources(&sources).unwrap();
    let altered = BTreeMap::from([(
        "actual-null.qv".into(),
        format!("{SOURCE}\n// different selected bytes\n"),
    )]);
    assert!(qcore_audit::null_hypotheses::View::new(&environment, &altered).is_err());
}

#[test]
fn actual_resource_callback_contract_propagates_through_a_real_summary_without_changing_classification()
 {
    use qcore_c_gateway::contract_dependencies::{Recording, hypothesis_closure, row_dependencies};
    use qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit;
    const CLIENT: &str =
        include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv");
    let erased = CLIENT
        .lines()
        .filter(|line| {
            let line = line.trim();
            !line.starts_with("quant proposition #plugin_")
                && !line.starts_with("requires resource_hypothesis(plugin_")
                && !line.starts_with(
                    "requires resource_premise_decomposition(rk_plugin_callback_closure,",
                )
                // Dependent symbolic metadata must be erased with its graph atoms.
                && !line.starts_with("requires resource_lifecycle_v1(introduce_plugin_")
                && !line.starts_with(
                    "requires resource_premise_flow_v1(decompose_rk_plugin_callback_closure,",
                )
                && !line.starts_with("// qcore-premise-binding-manifest-v1 ")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let original = qcore_contract::contract_loader::load_resource_contract_sources(
        &BTreeMap::from([("client.qv".into(), erased)]),
    )
    .unwrap();
    let current = qcore_contract::contract_loader::load_resource_contract_sources(&BTreeMap::from(
        [("client.qv".into(), CLIENT.into())],
    ))
    .unwrap();
    let source = "int direct(void *queue) { return rd_kafka_q_purge0(queue, 0); } int relay(void *queue) { return direct(queue); } int unrelated(void) { return 7; }";
    let old = analyze_legacy_c_translation_unit(source, "relay", &original).unwrap();
    let recording = Recording::start();
    let new = analyze_legacy_c_translation_unit(source, "relay", &current).unwrap();
    let record = recording.finish();
    let rows = row_dependencies(&record, &new, &current);
    let graphs = hypothesis::replay(current.source_assumptions()).unwrap();
    for name in ["direct", "relay", "unrelated"] {
        assert_eq!(
            new.functions[name].status, old.functions[name].status,
            "{name}"
        );
    }
    for name in ["direct", "relay"] {
        let closure = hypothesis_closure(&rows[name], &graphs);
        let selected = &closure["rd_kafka_q_purge0"];
        assert!(selected.contains("plugin_interceptor_registration"));
        assert!(selected.contains("plugin_code_context_lifetime"));
        assert_eq!(
            selected.len(),
            9,
            "seven proposed atoms plus two unchanged legacy atoms"
        );
    }
    assert!(rows["relay"].summaries.contains("direct"));
    assert!(hypothesis_closure(&rows["unrelated"], &graphs).is_empty());
    let dependencies = json!({"declarations":["rd_kafka_q_purge0"]});
    let before = qcore_audit::outer_selection::custody_facts(
        &dependencies,
        &qcore_audit::contract_dependencies::premises(&original).unwrap(),
    );
    let after = qcore_audit::outer_selection::custody_facts(
        &dependencies,
        &qcore_audit::contract_dependencies::premises(&current).unwrap(),
    );
    assert_eq!(
        before, after,
        "full original closed-world callback facts remain exact"
    );
}

#[test]
fn actual_clang_member_reader_keeps_the_full_null_row_and_routes_its_real_premise() {
    use qcore_c_gateway::legacy_c::C11TranslationUnit;
    use qcore_c_gateway::null_deref::analyze_null_c11;
    use qcore_schemas::contract::ResourceContracts;
    let erased = SOURCE
        .lines()
        .filter(|line| {
            let line = line.trim();
            !line.starts_with("quant proposition #plugin_")
                && !line.starts_with("requires resource_hypothesis(plugin_")
                && !line.starts_with(
                    "requires resource_premise_decomposition(rk_plugin_callback_closure,",
                )
                // Dependent symbolic metadata must be erased with its graph atoms.
                && !line.starts_with("requires resource_lifecycle_v1(introduce_plugin_")
                && !line.starts_with(
                    "requires resource_premise_flow_v1(decompose_rk_plugin_callback_closure,",
                )
                && !line.starts_with("// qcore-premise-binding-manifest-v1 ")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let before = bounds_contract::load_bounds_contract_sources(&BTreeMap::from([(
        "actual-null.qv".into(),
        erased,
    )]))
    .unwrap();
    let sources = BTreeMap::from([("actual-null.qv".into(), SOURCE.into())]);
    let after = bounds_contract::load_bounds_contract_sources(&sources).unwrap();
    let source = "struct rd_kafka_topic_s { char *rkt_topic; }; int reader(struct rd_kafka_topic_s *p) { if (!p) return 0; return p->rkt_topic[0]; } int unrelated(void) { return 7; }";
    let unit = C11TranslationUnit {
        source,
        ..Default::default()
    };
    let functions = vec!["reader".into(), "unrelated".into()];
    let resource = ResourceContracts::default();
    let old = analyze_null_c11(&unit, &functions, &resource, &before).unwrap();
    let new = analyze_null_c11(&unit, &functions, &resource, &after).unwrap();
    let view = qcore_audit::null_hypotheses::View::new(&after, &sources).unwrap();
    for name in ["reader", "unrelated"] {
        let actual = new.row(name).unwrap();
        assert_eq!(actual, old.row(name).unwrap(), "complete null row {name}");
        let records = actual
            .premises
            .iter()
            .map(|p| ErasedPremise {
                name: p.name.clone(),
                statement: p.statement.clone(),
                origin: "declared".into(),
            })
            .collect::<Vec<_>>();
        let mut graph = hypothesis::legacy_records(&records).unwrap();
        let references = view.attach(&json!({"null":actual}), &mut graph).unwrap();
        if name == "reader" {
            assert!(actual.premises.iter().any(|p| p.name == PREMISE));
            assert_eq!(graph.refinements.len(), 1);
            assert_eq!(references.len(), 1);
            assert_eq!(references[0].declaration, "rd_kafka_topic_new0");
        } else {
            assert!(graph.refinements.is_empty());
            assert!(references.is_empty());
        }
    }
}
