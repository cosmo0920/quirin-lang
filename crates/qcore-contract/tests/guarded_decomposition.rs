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

use qcore_contract::{guarded_decomposition as guarded, hypothesis};
use qcore_schemas::{
    hypothesis::{Context, Disposition, Graph, Warrant},
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
fn application(
    name: &str,
) -> (
    BTreeMap<String, String>,
    String,
    String,
    Graph,
    BTreeMap<String, guarded::Binding>,
    String,
) {
    let dir = root().join("docs/reports/premise-guarded-decomposition/applications");
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join(format!("{name}.json"))).unwrap()).unwrap();
    let module = metadata["module"].as_str().unwrap().to_string();
    let declaration = metadata["declaration"].as_str().unwrap().to_string();
    let mut source = fs::read_to_string(root().join(&module)).unwrap();
    if name == "plugin" {
        // This application owns its opaque checkpoint. The typed migration
        // preserves its exact source as an independent, hash-checked asset.
        source = fs::read_to_string(
            root()
                .join("docs/reports/quant-contract-hypotheses/plugin/before")
                .join(&module),
        )
        .unwrap();
    }
    if name == "cjson" {
        // The frozen sidecar checkpoint retains its original source identity.
        // Recover that exact source only through the checked version-2 bridge.
        let plan_bytes =
            fs::read(root().join("docs/reports/guarded-contract-adoption/adoption-plan.json"))
                .unwrap();
        let plan: qcore_contract::guarded_metadata_bridge::Plan =
            serde_json::from_slice(&plan_bytes).unwrap();
        let addition = qcore_contract::guarded_metadata_bridge::render(&plan.metadata).unwrap();
        let candidate = source.replacen(&addition, "", 1);
        let tip = BTreeMap::from([(module.clone(), source)]);
        let old = BTreeMap::from([(module.clone(), candidate)]);
        let checked = qcore_contract::guarded_metadata_bridge::check(
            &plan_bytes,
            &qcore_schemas::sha256_hex(&plan_bytes),
            &old,
            &tip,
        )
        .unwrap();
        source = checked
            .recover_before(&tip)
            .unwrap()
            .remove(&module)
            .unwrap();
    }
    assert_eq!(
        qcore_schemas::sha256_hex(source.as_bytes()),
        metadata["source_sha256"]
    );
    assert_eq!(metadata["source_binding"], "open");
    for reference in metadata["provenance"].as_array().unwrap() {
        assert_eq!(
            qcore_schemas::sha256_hex(
                &fs::read(root().join(reference["path"].as_str().unwrap())).unwrap()
            ),
            reference["sha256"]
        );
    }
    (
        BTreeMap::from([(module.clone(), source)]),
        module,
        declaration,
        serde_json::from_slice(&fs::read(dir.join(format!("{name}-graph.json"))).unwrap()).unwrap(),
        serde_json::from_value(metadata["bindings"].clone()).unwrap(),
        fs::read_to_string(dir.join(format!("{name}.qgd"))).unwrap(),
    )
}
fn document(name: &str) -> guarded::Lowered {
    let (sources, module, decl, graph, bindings, readable) = application(name);
    guarded::replay(
        View::Resource,
        &sources,
        &module,
        &decl,
        &graph,
        &bindings,
        &readable,
    )
    .unwrap()
}
fn context() -> Context {
    Context {
        artifact: "artifact".into(),
        property: "property".into(),
        semantic_profile: "profile".into(),
        scope: "scope".into(),
        bounds: "bounds".into(),
        assumptions: "assumptions".into(),
        source_identity: "source".into(),
        build_identity: "build".into(),
        evidence_version: "evidence".into(),
    }
}
fn assignments(d: &guarded::Lowered, values: &[(&str, Disposition)]) -> Vec<guarded::Assignment> {
    values
        .iter()
        .map(|(symbol, disposition)| guarded::Assignment {
            symbol: (*symbol).into(),
            symbol_sha256: d.symbol_sha256(symbol).unwrap(),
            context: context(),
            disposition: *disposition,
            warrant: Warrant::Declared,
        })
        .collect()
}
fn evaluate(d: &guarded::Lowered, values: &[(&str, Disposition)]) -> guarded::ProposedEvaluation {
    guarded::evaluate_proposed(d, &context(), &context(), &assignments(d, values)).unwrap()
}

#[test]
fn actual_cjson_and_plugin_hypotheses_lower_without_truth_or_source_changes() {
    for (name, atoms) in [("cjson", 9), ("plugin", 7)] {
        let (sources, module, decl, graph, bindings, readable) = application(name);
        let graphs_before = hypothesis::replay(&sources).unwrap();
        let loaded_before =
            qcore_contract::contract_loader::load_resource_contract_sources(&sources).unwrap();
        let d = guarded::replay(
            View::Resource,
            &sources,
            &module,
            &decl,
            &graph,
            &bindings,
            &readable,
        )
        .unwrap();
        assert_eq!(d.guards().len(), 2);
        assert_eq!(d.alternatives().len(), 1);
        assert_eq!(d.terminal_postconditions().len(), 2);
        let prerequisites: Vec<_> = bindings
            .values()
            .filter(|b| b.role == guarded::Role::Prerequisite && b.dependencies.len() == 1)
            .collect();
        assert_eq!(prerequisites.len(), atoms);
        for b in prerequisites {
            let atom = &graph.atoms[&b.dependencies[0]];
            let qcore_schemas::hypothesis::Proposition::Opaque { statement } = &atom.proposition
            else {
                panic!()
            };
            assert_eq!(&b.proposition, statement);
        }
        let e = evaluate(&d, &[]);
        assert_eq!(e.guards, vec![guarded::Status::Open; 2]);
        assert_eq!(e.alternatives, vec![guarded::Status::Open]);
        assert_eq!(e.terminal_postconditions, vec![guarded::Status::Open; 2]);
        assert_eq!(e.dependencies.len(), bindings.len());
        assert_eq!(hypothesis::replay(&sources).unwrap(), graphs_before);
        assert_eq!(
            qcore_contract::contract_loader::load_resource_contract_sources(&sources)
                .unwrap()
                .functions(),
            loaded_before.functions()
        );
    }
}

#[test]
fn forbidden_requires_positive_invalidity_and_positive_operation_refutation() {
    use Disposition::*;
    use guarded::Status;
    let d = document("cjson");
    assert_eq!(
        evaluate(&d, &[("invalid_membership", Supported)]).guards[1],
        Status::Open
    );
    assert_eq!(
        evaluate(
            &d,
            &[("invalid_membership", Supported), ("detach", Refuted)]
        )
        .guards[1],
        Status::Satisfied
    );
    assert_eq!(
        evaluate(
            &d,
            &[("invalid_membership", Supported), ("detach", Supported)]
        )
        .guards[1],
        Status::Refuted
    );
    assert_eq!(
        evaluate(
            &d,
            &[("invalid_membership", Refuted), ("detach", Supported)]
        )
        .guards[1],
        Status::Inactive
    );
    assert_eq!(evaluate(&d, &[("detach", Refuted)]).guards[1], Status::Open);
}

#[test]
fn positive_success_does_not_hide_missing_or_refuted_failure_postcondition() {
    use Disposition::*;
    use guarded::Status;
    let d = document("cjson");
    let e = evaluate(
        &d,
        &[
            ("detach", Supported),
            ("success_case", Supported),
            ("remaining_list_valid", Supported),
        ],
    );
    assert_eq!(e.alternatives, vec![Status::Satisfied]);
    assert_eq!(
        e.terminal_postconditions,
        vec![Status::Satisfied, Status::Open]
    );
    assert_eq!(e.guards[0], Status::Open); // a terminal establishes no entry guard
    let e = evaluate(
        &d,
        &[
            ("detach", Supported),
            ("success_case", Supported),
            ("failure_case", Supported),
            ("original_list_unchanged", Refuted),
        ],
    );
    assert_eq!(
        e.terminal_postconditions,
        vec![Status::Open, Status::Refuted]
    );
    assert_eq!(e.dependencies.len(), d.bindings().len());
}

#[test]
fn explicit_version_grammar_roles_and_complete_cases_refuse_total_or_reject() {
    let (sources, module, decl, graph, bindings, readable) = application("cjson");
    let invalid = [
        readable.replace("guarded_decomposition_v1", "guarded_decomposition_v2"),
        readable.replace(" ∧ ", " & "),
        readable.replace("|-> detach", "| -> detach"),
        readable.replace("!-> detach", "! -> detach"),
        readable.replace("||-> success_case / failure_case", "||-> success_case"),
        readable.replace(
            "||-> success_case / failure_case",
            "||-> success_case / success_case",
        ),
        readable.replace("failure_case ->| original_list_unchanged;", ""),
        readable.replace(
            "success_case ->| remaining_list_valid;",
            "success_case ->| remaining_list_valid; success_case ->| original_list_unchanged;",
        ),
        readable.replace("environment ∧", "success_case ∧"),
        readable.replace("!-> detach", "!-> remaining_list_valid"),
        readable.replace(" / failure_case", " / undeclared"),
        readable.replace(" ->| ", " -> | "),
        format!("{readable};"),
        readable.trim_end().trim_end_matches(';').to_string(),
        format!("{readable}garbage;"),
        format!("{readable}invalid_membership !-> detach;"),
        readable.replace("(environment ∧", "environment ∧"),
        readable.replace("detach_distinct ∧", "detach_chain ∧"),
        format!("guarded_decomposition_v1;{}", "x".repeat(65536)),
    ];
    for (i, text) in invalid.iter().enumerate() {
        assert!(
            guarded::replay(
                View::Resource,
                &sources,
                &module,
                &decl,
                &graph,
                &bindings,
                text
            )
            .is_err(),
            "negative {i}"
        );
    }
    let mut wrong = bindings.clone();
    wrong.get_mut("detach").unwrap().dependencies = vec!["undeclared".into()];
    assert!(
        guarded::replay(
            View::Resource,
            &sources,
            &module,
            &decl,
            &graph,
            &wrong,
            &readable
        )
        .is_err()
    );
    wrong = bindings.clone();
    wrong.insert("unused".into(), bindings["detach"].clone());
    assert!(
        guarded::replay(
            View::Resource,
            &sources,
            &module,
            &decl,
            &graph,
            &wrong,
            &readable
        )
        .is_err()
    );
    let mut wrong_graph = graph.clone();
    wrong_graph.atoms.remove("detach_chain");
    assert!(
        guarded::replay(
            View::Resource,
            &sources,
            &module,
            &decl,
            &wrong_graph,
            &bindings,
            &readable
        )
        .is_err()
    );
    assert!(
        guarded::replay(
            View::Bounds,
            &sources,
            &module,
            &decl,
            &graph,
            &bindings,
            &readable
        )
        .is_err()
    );
    let mut malformed = sources.clone();
    malformed
        .get_mut(&module)
        .unwrap()
        .push_str("\nthis does not parse\n");
    assert!(
        guarded::replay(
            View::Resource,
            &malformed,
            &module,
            &decl,
            &graph,
            &bindings,
            &readable
        )
        .is_err()
    );
    assert!(
        guarded::replay(
            View::Resource,
            &sources,
            "missing",
            &decl,
            &graph,
            &bindings,
            &readable
        )
        .is_err()
    );
}

#[test]
fn all_nine_context_coordinates_and_every_alternative_assignment_are_bound() {
    let d = document("cjson");
    let expected = context();
    let json = serde_json::to_value(&expected).unwrap();
    for key in json.as_object().unwrap().keys() {
        let mut wrong = json.clone();
        wrong[key] = serde_json::Value::String("mismatch".into());
        let wrong: Context = serde_json::from_value(wrong).unwrap();
        assert!(guarded::evaluate_proposed(&d, &expected, &wrong, &[]).is_err());
        assert!(
            guarded::affected_obligations(&d, &expected, &wrong, &["environment".into()]).is_err()
        );
        let mut evidence = assignments(&d, &[("success_case", Disposition::Supported)]);
        evidence[0].context = wrong;
        assert!(guarded::evaluate_proposed(&d, &expected, &expected, &evidence).is_err());
    }
    let mut evidence = assignments(
        &d,
        &[
            ("success_case", Disposition::Supported),
            ("failure_case", Disposition::Supported),
        ],
    );
    evidence[1].symbol_sha256 = "wrong-unchosen-case".into();
    assert!(guarded::evaluate_proposed(&d, &expected, &expected, &evidence).is_err());
    let mut evidence = assignments(&d, &[("success_case", Disposition::Supported)]);
    for warrant in [Warrant::Derived, Warrant::ProviderProposed] {
        evidence[0].warrant = warrant;
        assert!(guarded::evaluate_proposed(&d, &expected, &expected, &evidence).is_err());
    }
    let evidence = assignments(
        &d,
        &[
            ("success_case", Disposition::Supported),
            ("success_case", Disposition::Refuted),
        ],
    );
    assert!(guarded::evaluate_proposed(&d, &expected, &expected, &evidence).is_err());
}

#[test]
fn exact_source_and_model_bytes_bind_even_semantically_equal_documents() {
    let (sources, module, decl, graph, bindings, readable) = application("cjson");
    let d = guarded::replay(
        View::Resource,
        &sources,
        &module,
        &decl,
        &graph,
        &bindings,
        &readable,
    )
    .unwrap();
    let mut different = sources.clone();
    different.get_mut(&module).unwrap().push('\n');
    let changed = guarded::replay(
        View::Resource,
        &different,
        &module,
        &decl,
        &graph,
        &bindings,
        &readable,
    )
    .unwrap();
    assert_ne!(d.source_sha256(), changed.source_sha256());
    assert_eq!(d.graph_sha256(), changed.graph_sha256());
    let evidence = assignments(&d, &[("success_case", Disposition::Supported)]);
    assert!(guarded::evaluate_proposed(&changed, &context(), &context(), &evidence).is_err());
    let changed = guarded::replay(
        View::Resource,
        &sources,
        &module,
        &decl,
        &graph,
        &bindings,
        &format!("{readable}\n"),
    )
    .unwrap();
    assert!(guarded::evaluate_proposed(&changed, &context(), &context(), &evidence).is_err());
    let mut different = bindings;
    different
        .get_mut("failure_case")
        .unwrap()
        .proposition
        .push_str(" Changed.");
    let changed = guarded::replay(
        View::Resource,
        &sources,
        &module,
        &decl,
        &graph,
        &different,
        &readable,
    )
    .unwrap();
    assert!(guarded::evaluate_proposed(&changed, &context(), &context(), &evidence).is_err());
}

#[test]
fn prerequisite_errors_propagate_to_every_case_but_terminal_errors_never_enable_calls() {
    let d = document("cjson");
    let impact =
        guarded::affected_obligations(&d, &context(), &context(), &["detach_chain".into()])
            .unwrap();
    assert_eq!(impact.alternative_indices, vec![0]);
    assert_eq!(impact.terminal_indices, vec![0, 1]);
    for symbol in [
        "detach_chain",
        "detach",
        "success_case",
        "failure_case",
        "remaining_list_valid",
        "original_list_unchanged",
    ] {
        assert!(impact.symbols_requiring_recheck.contains(&symbol.into()));
    }
    let impact = guarded::affected_obligations(
        &d,
        &context(),
        &context(),
        &["original_list_unchanged".into()],
    )
    .unwrap();
    assert!(impact.guard_indices.is_empty());
    assert!(impact.alternative_indices.is_empty());
    assert_eq!(impact.terminal_indices, vec![1]);
    assert!(
        guarded::affected_obligations(&d, &context(), &context(), &["unknown".into()]).is_err()
    );
    assert!(
        guarded::affected_obligations(
            &d,
            &context(),
            &context(),
            &["detach".into(), "detach".into()]
        )
        .is_err()
    );
    assert_eq!(evaluate(&d, &[]).guards, vec![guarded::Status::Open; 2]);
}

#[test]
fn operation_admission_requires_entry_prerequisites_in_the_correct_direction() {
    use Disposition::*;
    use guarded::Status;
    let d = document("cjson");
    // Positive admission with a positively refuted prerequisite is contradicted,
    // rather than made inactive by the false prerequisite on the left.
    assert_eq!(
        evaluate(&d, &[("detach", Supported), ("detach_chain", Refuted)]).guards[0],
        Status::Refuted
    );
    assert_eq!(
        evaluate(&d, &[("detach", Supported)]).guards[0],
        Status::Open
    );
    let mut values: Vec<_> = d
        .bindings()
        .iter()
        .filter(|(_, b)| {
            matches!(
                b.role,
                guarded::Role::Environment | guarded::Role::Prerequisite
            )
        })
        .map(|(s, _)| (s.as_str(), Supported))
        .collect();
    // Entry facts alone do not assert that an operation occurs or is admitted.
    assert_eq!(evaluate(&d, &values).guards[0], Status::Open);
    values.push(("detach", Supported));
    assert_eq!(evaluate(&d, &values).guards[0], Status::Satisfied);
}
