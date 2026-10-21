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
    let row = json!({"artifact":"caller","null":{"state":"Safe","contract_dependencies":["member:rd_kafka_topic_s.rkt_topic"],"premises":[{"name":target.erasure.name,"statement":target.erasure.statement,"through":["member:rd_kafka_topic_s.rkt_topic"]}]}});
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
    let mut missing = row.clone();
    missing["null"]["contract_dependencies"] = json!([]);
    assert!(view.attach(&missing, &mut original.clone()).is_err());
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
