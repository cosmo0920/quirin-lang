use qcore_contract::{guarded_decomposition as g, guarded_metadata_bridge as bridge, hypothesis};
use qcore_schemas::{
    hypothesis::{Context, Disposition, Warrant},
    premise_flow::View,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn fixture() -> (
    Vec<u8>,
    bridge::Plan,
    BTreeMap<String, String>,
    BTreeMap<String, String>,
) {
    let bytes =
        fs::read(root().join("docs/reports/guarded-contract-adoption/adoption-plan.json")).unwrap();
    let plan: bridge::Plan = serde_json::from_slice(&bytes).unwrap();
    let source = fs::read_to_string(root().join(&plan.module)).unwrap();
    let addition = bridge::render(&plan.metadata).unwrap();
    assert_eq!(source.matches(&addition).count(), 1);
    let before = BTreeMap::from([(plan.module.clone(), source.replacen(&addition, "", 1))]);
    let after = BTreeMap::from([(plan.module.clone(), source)]);
    (bytes, plan, before, after)
}
fn context() -> Context {
    Context {
        artifact: "a".into(),
        property: "resource".into(),
        semantic_profile: "p".into(),
        scope: "s".into(),
        bounds: "b".into(),
        assumptions: "x".into(),
        source_identity: "i".into(),
        build_identity: "j".into(),
        evidence_version: "e".into(),
    }
}
#[test]
fn actual_production_source_has_complete_independently_expected_map_and_no_truth_promotion() {
    let (bytes, p, before, after) = fixture();
    let checked =
        bridge::check(&bytes, &qcore_schemas::sha256_hex(&bytes), &before, &after).unwrap();
    assert_eq!(checked.recover_before(&after).unwrap(), before);
    assert_eq!(
        hypothesis::replay(&before).unwrap(),
        hypothesis::replay(&after).unwrap()
    );
    let sidecar: serde_json::Value = serde_json::from_slice(
        &fs::read(
            root().join("docs/reports/premise-guarded-decomposition/applications/cjson.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&p.metadata.bindings).unwrap(),
        sidecar["bindings"]
    );
    assert_eq!(
        p.metadata.readable,
        fs::read_to_string(
            root().join("docs/reports/premise-guarded-decomposition/applications/cjson.qgd")
        )
        .unwrap()
    );
    assert_eq!(p.metadata.bindings.len(), 17);
    let d = g::replay_inline(
        View::Resource,
        &after,
        &p.module,
        &p.declaration,
        &p.graph,
        &p.metadata,
    )
    .unwrap();
    let status = g::evaluate_proposed(&d, &context(), &context(), &[]).unwrap();
    assert!(
        status
            .guards
            .iter()
            .chain(&status.alternatives)
            .chain(&status.terminal_postconditions)
            .all(|s| *s == g::Status::Open)
    );
    assert_eq!(
        (
            d.guards().len(),
            d.alternatives().len(),
            d.terminal_postconditions().len()
        ),
        (2, 1, 2)
    );
}
#[test]
fn every_non_metadata_or_noncanonical_source_change_refuses_whole_source_bridge() {
    let (bytes, p, before, after) = fixture();
    let sha = qcore_schemas::sha256_hex(&bytes);
    for edited in [
        after[&p.module].replace(
            "resource_owned(parent, nonnull)",
            "resource_owned(parent, nullable)",
        ),
        after[&p.module].replace(
            "invalid_membership !-> detach",
            "invalid_membership |-> detach",
        ),
        after[&p.module].replace("success_case / failure_case", "success_case / success_case"),
        after[&p.module].replace(
            "    requires resource_guarded_symbol_v1(detach,",
            "    ensures normal resource_guarded_symbol_v1(detach,",
        ),
        after[&p.module].replace(
            "resource_guarded_decomposition_v1",
            "resource_guarded_decomposition_v2",
        ),
        after[&p.module].replace("\n", "\r\n"),
        after[&p.module].clone()
            + "// unrelated byte
",
    ] {
        assert_ne!(edited, after[&p.module], "each mutation must change bytes");
        let mutated = BTreeMap::from([(p.module.clone(), edited)]);
        assert!(bridge::check(&bytes, &sha, &before, &mutated).is_err());
    }
    assert!(bridge::check(&bytes, &"0".repeat(64), &before, &after).is_err());
    let mut excess = after.clone();
    excess.insert("extra.qv".into(), "module contract;".into());
    assert!(bridge::check(&bytes, &sha, &before, &excess).is_err());
    let checked = bridge::check(&bytes, &sha, &before, &after).unwrap();
    assert!(checked.recover_before(&excess).is_err());
    assert!(checked.recover_before(&before).is_err());
}
#[test]
fn refreshed_attack_plan_cannot_erase_changed_interpreted_output_graph_or_proposition() {
    let (_, p, before, after) = fixture();
    for edit in ["semantic", "graph", "binding", "ordering", "version"] {
        let mut attack = p.clone();
        let mut target = after.clone();
        match edit {
            "semantic" => {
                let v = target.get_mut(&p.module).unwrap();
                *v = v.replace(
                    "resource_owned(parent, nonnull)",
                    "resource_owned(parent, nullable)",
                );
            }
            "graph" => {
                attack.graph.atoms.clear();
            }
            "binding" => {
                attack
                    .metadata
                    .bindings
                    .get_mut("detach")
                    .unwrap()
                    .proposition
                    .push_str(" changed");
            }
            "ordering" => {
                attack
                    .metadata
                    .bindings
                    .get_mut("detach")
                    .unwrap()
                    .dependencies
                    .reverse();
            }
            _ => attack.version = 3,
        }
        attack.after_sha256 = qcore_schemas::sha256_hex(target[&p.module].as_bytes());
        let bytes = serde_json::to_vec(&attack).unwrap();
        assert!(
            bridge::check(&bytes, &qcore_schemas::sha256_hex(&bytes), &before, &target).is_err(),
            "{edit}"
        );
    }
}
#[test]
fn loaders_reject_every_malformed_inline_boundary_including_unchosen_case() {
    let (_, p, _, after) = fixture();
    for edited in [
        after[&p.module].replace(
            "failure_case ->| original_list_unchanged;",
            "failure_case ->| absent;",
        ),
        after[&p.module].replace("failure_case, case,", "failure_case, operation,"),
        after[&p.module].replace("detach, operation,", "detach, mystery,"),
        after[&p.module].replace("detach_chain, prerequisite,", "detach_chain, environment,"),
        after[&p.module].replace(
            "resource_guarded_decomposition_v1",
            "resource_guarded_decomposition_v0",
        ),
        after[&p.module].replace(
            "invalid_membership !-> detach;",
            "invalid_membership ! -> detach;",
        ),
        after[&p.module].replace(
            "success_case / failure_case",
            "success_case / failure_case / failure_case",
        ),
        after[&p.module].replace("environment ∧ detach_chain", "environment ∧ missing_atom"),
    ] {
        let bad = BTreeMap::from([(p.module.clone(), edited)]);
        assert!(qcore_contract::contract_loader::load_resource_contract_sources(&bad).is_err());
        assert!(hypothesis::replay(&bad).is_err());
    }
    let mut badgraph = p.graph.clone();
    badgraph.atoms.clear();
    assert!(
        g::replay_inline(
            View::Resource,
            &after,
            &p.module,
            &p.declaration,
            &badgraph,
            &p.metadata
        )
        .is_err()
    );
    assert!(
        g::replay_inline(
            View::Bounds,
            &after,
            &p.module,
            &p.declaration,
            &p.graph,
            &p.metadata
        )
        .is_err()
    );
}
#[test]
fn source_identity_and_all_context_coordinates_bind_proposed_evidence() {
    let (_, p, _, after) = fixture();
    let d = g::replay_inline(
        View::Resource,
        &after,
        &p.module,
        &p.declaration,
        &p.graph,
        &p.metadata,
    )
    .unwrap();
    let mut changed = after.clone();
    changed.get_mut(&p.module).unwrap().push_str(
        "
",
    );
    let other = g::replay_inline(
        View::Resource,
        &changed,
        &p.module,
        &p.declaration,
        &p.graph,
        &p.metadata,
    )
    .unwrap();
    assert_ne!(
        d.symbol_sha256("detach").unwrap(),
        other.symbol_sha256("detach").unwrap()
    );
    let assignment = g::Assignment {
        symbol: "detach".into(),
        symbol_sha256: other.symbol_sha256("detach").unwrap(),
        context: context(),
        disposition: Disposition::Supported,
        warrant: Warrant::Declared,
    };
    assert!(g::evaluate_proposed(&d, &context(), &context(), &[assignment]).is_err());
    for key in [
        "artifact",
        "property",
        "semantic_profile",
        "scope",
        "bounds",
        "assumptions",
        "source_identity",
        "build_identity",
        "evidence_version",
    ] {
        let mut wire = serde_json::to_value(context()).unwrap();
        wire[key] = serde_json::json!("foreign");
        let c = serde_json::from_value(wire).unwrap();
        assert!(
            g::evaluate_proposed(&d, &context(), &c, &[]).is_err(),
            "{key}"
        );
        assert!(
            g::affected_obligations(&d, &context(), &c, &["environment".into()]).is_err(),
            "{key}"
        );
    }
}
