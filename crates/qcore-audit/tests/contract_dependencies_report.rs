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

//! Card **VERDICT-CONTRACT-DEPENDENCIES** (rule 13h): the run's dependency
//! section and the shared `standingVectors` of
//! `proof/QCore/ContractDependencies.lean`; card
//! **VERDICT-BODY-STANDING-CLAUSE-SCOPE** (rules 13i–13k): the per-clause
//! standing, the partly-validated bucket and the shared
//! `clauseStandingVectors` of `proof/QCore/BodyClauseStanding.lean`.

use std::collections::BTreeMap;

use qcore_audit::contract_dependencies::{
    CLAUSES_KEY, Facts, PREMISES_KEY, ROW_KEY, SECTION_KEY, UNIT_KEY, body_standing_of,
    clause_standing_of, premises, render,
};
use qcore_c_gateway::contract_clause_standing::covers;
use serde_json::{Value, json};

const LEAN: &str = include_str!("../../../proof/QCore/ContractDependencies.lean");
const CLAUSE_LEAN: &str = include_str!("../../../proof/QCore/BodyClauseStanding.lean");

#[test]
fn lean_standing_vectors_match_the_run_standing() {
    let header = "def standingVectors : List (Bool × Nat × Nat × Nat × String) := [";
    let start = LEAN.find(header).expect("the Lean list exists") + header.len();
    let body = &LEAN[start..];
    let rows = body[..body.find("\n]").expect("closes")]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            line.trim_start_matches('(')
                .trim_end_matches(',')
                .trim_end_matches(')')
                .split(',')
                .map(|field| field.trim().trim_matches('"').to_owned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 6);
    for row in rows {
        let status = (row[0] == "true").then_some("pending-review");
        let count = |field: &str| field.parse::<usize>().expect("a count");
        assert_eq!(
            body_standing_of(status, count(&row[1]), count(&row[2]), count(&row[3])),
            row[4],
            "{row:?}"
        );
    }
}

fn function(name: &str, status: &str, field: Value) -> Value {
    json!({"function": name, "status": status, "diagnostics": [], ROW_KEY: field})
}

#[test]
fn the_section_rolls_up_the_safe_rows_by_what_they_rest_on() {
    let units = vec![
        json!({
            "library": "l", "file": "a.c",
            "functions": [
                function("only_validated", "Safe", json!({"declarations": ["v"]})),
                function("on_assumption", "Safe", json!({"declarations": ["v", "a"]})),
                function("nothing", "Safe", json!({"declarations": []})),
                function("held", "Safe", json!({"declarations": [], "held": ["h"]})),
                function("unknown", "Unknown", json!({"declarations": ["a"]})),
            ],
            UNIT_KEY: {"validated": {"v": ["W4"]}, "unvalidated": ["a"], "failed": {}, "withdrawn": {},
                       CLAUSES_KEY: {"v": {"resource_released(#0, nonnull)": ["W4"],
                                           "resource_return_none()": ["W4"]}}},
        }),
        json!({
            "library": "l", "file": "b.c",
            "functions": [function("again", "Safe", json!({"declarations": ["v"]}))],
            UNIT_KEY: {"validated": {"v": ["entry"]}, "unvalidated": [], "failed": {}, "withdrawn": {},
                       CLAUSES_KEY: {"v": {"resource_released(#0, nonnull)": ["entry"],
                                           "resource_return_none()": ["entry"]}}},
        }),
    ];
    // The fold of the two units equals the absorb of their own folds.
    let mut absorbed = Facts::of_rows(&units[..1]);
    absorbed.absorb(Facts::of_rows(&units[1..]));
    assert_eq!(absorbed, Facts::of_rows(&units));
    let revision = json!({"contracts": {
        "v": {"mode": "pinned", "status": "pinned-in-force", "contract_sha256": "11", "module": "m"},
        "a": {"mode": "flexible", "status": "flexible-standing", "contract_sha256": "22", "module": "m"},
        "h": {"mode": "pinned", "status": "pending-review", "contract_sha256": "33", "module": "m"},
    }});
    let premises = json!({"a": [{"name": "served_queue", "statement": "s", "origin": "declared"}]});
    let section = render(&absorbed, &revision, &premises);
    let rollup = &section["safe_rollup"];
    assert_eq!(rollup["safe_rows"], 5);
    assert_eq!(rollup["resting_only_on_validated_declarations"], 2);
    assert_eq!(rollup["resting_on_a_declared_assumption"], 1);
    assert_eq!(rollup["resting_on_no_declaration"], 1);
    assert_eq!(rollup["consulting_a_held_or_withdrawn_declaration"], 1);
    assert_eq!(
        rollup["top_declared_assumptions"],
        json!([{"function": "a", "safe_rows": 1, "mode": "flexible",
                 "status": "flexible-standing",
                 "premises": [{"name": "served_queue", "statement": "s", "origin": "declared"}]}])
    );
    let declarations = &section["declarations"];
    assert_eq!(declarations["v"]["body_standing"], "validated");
    assert_eq!(
        declarations["v"]["bodies"]["validated_by"],
        json!(["W4", "entry"])
    );
    assert_eq!(declarations["a"]["body_standing"], "declared-assumption");
    assert_eq!(declarations["h"]["body_standing"], "held");
    assert_eq!(declarations["a"]["rows"], 2);
    assert_eq!(declarations["a"]["safe_rows"], 1);
    assert_eq!(
        declarations["v"]["clauses"],
        json!({"resource_released(#0, nonnull)": "validated", "resource_return_none()": "validated"})
    );
    assert_eq!(section["counts"]["partly_validated"], 0);
    assert_eq!(SECTION_KEY, "contract_dependencies");
    assert_eq!(PREMISES_KEY, "contract_premises");
}

#[test]
fn a_partly_validated_declaration_is_counted_apart_and_lists_its_gaps() {
    // `keep`: rule 13c covers its reference clause, nothing its borrowed
    // formal. `release`: W4 covers every clause. A row on `keep` (and
    // `release`) is not "only validated"; a row on `release` alone is.
    let units = vec![json!({
        "library": "l", "file": "a.c",
        "functions": [
            function("on_keep", "Safe", json!({"declarations": ["keep", "release"]})),
            function("on_release", "Safe", json!({"declarations": ["release"]})),
            function("on_both_and_an_assumption", "Safe",
                     json!({"declarations": ["keep", "assumed"]})),
        ],
        UNIT_KEY: {
            "validated": {"keep": ["13c"], "release": ["W4"]},
            "unvalidated": [], "failed": {}, "withdrawn": {},
            CLAUSES_KEY: {
                "keep": {"resource_borrowed(#0)": [], "resource_reference_kept(#0)": ["13c"],
                         "resource_return_alias(#0, nonnull)": []},
                "release": {"resource_released(#0, nonnull)": ["W4"], "resource_return_none()": ["W4"]},
            },
        },
    })];
    let revision = json!({"contracts": {
        "keep": {"mode": "pinned", "status": "pinned-in-force", "contract_sha256": "1", "module": "m"},
        "release": {"mode": "pinned", "status": "pinned-in-force", "contract_sha256": "2", "module": "m"},
        "assumed": {"mode": "pinned", "status": "pinned-in-force", "contract_sha256": "3", "module": "m"},
    }});
    let section = render(&Facts::of_rows(&units), &revision, &json!({}));
    let keep = &section["declarations"]["keep"];
    assert_eq!(keep["body_standing"], "partly-validated");
    assert_eq!(
        keep["uncovered_clauses"],
        json!([
            "resource_borrowed(#0)",
            "resource_return_alias(#0, nonnull)"
        ])
    );
    assert_eq!(keep["clauses"]["resource_reference_kept(#0)"], "validated");
    assert_eq!(
        section["declarations"]["release"]["body_standing"],
        "validated"
    );
    let rollup = &section["safe_rollup"];
    assert_eq!(rollup["resting_only_on_validated_declarations"], 1);
    assert_eq!(rollup["resting_on_a_partly_validated_declaration"], 1);
    assert_eq!(rollup["resting_on_a_declared_assumption"], 1);
    assert_eq!(
        rollup["partly_validated_declarations"],
        json!([{"function": "keep", "safe_rows": 2,
                 "uncovered_clauses": ["resource_borrowed(#0)", "resource_return_alias(#0, nonnull)"]}])
    );
    assert_eq!(section["counts"]["validated"], 1);
    assert_eq!(section["counts"]["partly_validated"], 1);
    // A unit row without clause coverage (a report written before rule 13j)
    // never makes a declaration validated.
    let mut old = units[0].clone();
    old[UNIT_KEY]
        .as_object_mut()
        .expect("unit field")
        .remove(CLAUSES_KEY);
    let section = render(&Facts::of_rows(&[old]), &revision, &json!({}));
    assert_eq!(
        section["declarations"]["release"]["body_standing"],
        "declared-assumption"
    );
}

/// The rows of `clauseStandingVectors`: (held, clause kinds, units, standing).
fn clause_standing_rows() -> Vec<(bool, Vec<String>, Vec<String>, String)> {
    let header =
        "def clauseStandingVectors : List (Bool × List String × List String × String) := [";
    let start = CLAUSE_LEAN.find(header).expect("the Lean list exists") + header.len();
    let body = &CLAUSE_LEAN[start..];
    let lists = |text: &str| {
        text.split(',')
            .map(|item| item.trim().trim_matches('"').to_owned())
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>()
    };
    body[..body.find("\n]").expect("closes")]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            let line = line.trim_start_matches('(').trim_end_matches(',');
            let line = line.strip_suffix(')').unwrap_or(line);
            let (held, rest) = line.split_once(", [").expect("clauses");
            let (clauses, rest) = rest.split_once("], [").expect("units");
            let (units, standing) = rest.split_once("], ").expect("standing");
            (
                held == "true",
                lists(clauses),
                lists(units),
                standing.trim_matches('"').to_owned(),
            )
        })
        .collect()
}

#[test]
fn lean_clause_standing_vectors_match_the_run_standing() {
    let rows = clause_standing_rows();
    // Card CG22-CALLBACK-REGISTRY-CONTRACT adds two rows (a checked callback
    // clause under `13y`, an unchecked one under `W4`): 19 -> 21; card
    // CG22-OP-CALLBACKS-AND-SERVED-QUEUES one (arms under `12o`): 22; card
    // OP-CONSTRUCTOR-DECLARATIONS two (a result state under `14e`, and under
    // `W4` alone): 24.
    assert_eq!(rows.len(), 24);
    for (held, clauses, units, expected) in rows {
        // Each unit of the vector as a unit row the audit folds, its clause
        // coverage computed by the gateway's `covers` (one label per kind).
        let rows = units
            .iter()
            .map(|unit| {
                let evidence = match unit.as_str() {
                    "failed" | "none" => Vec::new(),
                    spelled => spelled.split('+').map(str::to_owned).collect(),
                };
                let kind = |spelling: &str| {
                    qcore_c_gateway::contract_clause_standing::ClauseKind::ALL
                        .into_iter()
                        .find(|kind| kind.spelling() == spelling)
                        .expect("a clause kind")
                };
                let coverage = clauses
                    .iter()
                    .map(|clause| {
                        let by = evidence
                            .iter()
                            .filter(|by| covers(by, kind(clause)))
                            .cloned()
                            .collect::<Vec<_>>();
                        (clause.clone(), json!(by))
                    })
                    .collect::<serde_json::Map<_, _>>();
                let field = match unit.as_str() {
                    "failed" => json!({"validated": {}, "unvalidated": [],
                                       "failed": {"d": ["conflict"]}, "withdrawn": {}}),
                    "none" => json!({"validated": {}, "unvalidated": ["d"], "failed": {},
                                     "withdrawn": {}}),
                    _ => json!({"validated": {"d": evidence}, "unvalidated": [], "failed": {},
                                "withdrawn": {}, CLAUSES_KEY: {"d": coverage}}),
                };
                json!({"library": "l", "file": "u.c",
                       "functions": [function("f", "Safe", json!({"declarations": ["d"]}))],
                       UNIT_KEY: field})
            })
            .collect::<Vec<_>>();
        let status = if held {
            "pending-review"
        } else {
            "pinned-in-force"
        };
        let revision = json!({"contracts": {"d": {"mode": "pinned", "status": status,
                                                   "contract_sha256": "0", "module": "m"}}});
        let mut facts = Facts::of_rows(&rows);
        if rows.is_empty() {
            // A declaration consulted by a row, whose body no unit examined.
            facts = Facts::of_rows(&[json!({"library": "l", "file": "u.c",
                "functions": [function("f", "Safe", json!({"declarations": ["d"]}))]})]);
        }
        let section = render(&facts, &revision, &json!({}));
        assert_eq!(
            section["declarations"]["d"]["body_standing"], expected,
            "{held} {clauses:?} {units:?}"
        );
    }
    // The run standing's own entry point agrees on a direct case.
    let clauses = BTreeMap::from([("a".to_owned(), 2), ("b".to_owned(), 1)]);
    assert_eq!(
        clause_standing_of(None, 2, 0, 0, 2, &clauses),
        "partly-validated"
    );
    assert_eq!(
        clause_standing_of(None, 2, 0, 0, 1, &clauses),
        "declared-assumption"
    );
}

#[test]
fn the_premises_of_an_environment_name_declared_and_rule_premises() {
    let module = "module contract;

extern fn q_wait(q: ptr<mut, u8>, timeout: i32) -> i32
contract {
    requires resource_contract_v1;
    requires resource_premise(served_queue, \"no callback takes the caller's reference\");
    ensures normal resource_borrowed(q);
    ensures normal resource_borrowed(timeout);
    ensures normal resource_return_none();
};
";
    let contracts = qcore_contract::contract_loader::load_resource_contract_sources(
        &BTreeMap::from([("q.qv".to_owned(), module.to_owned())]),
    )
    .unwrap();
    let premises = premises(&contracts).unwrap();
    assert_eq!(
        premises["q_wait"],
        json!([{"name": "served_queue", "statement": "no callback takes the caller's reference",
                 "origin": "declared"}])
    );
}

#[test]
fn graph_section_erases_to_the_previous_premise_and_dependency_view() {
    use qcore_audit::contract_dependencies::erase_hypotheses;
    use qcore_contract::contract_loader::load_resource_contract_sources;
    let text = "module contract; extern fn f() -> unit contract { requires resource_contract_v1; requires resource_premise(p, \"assumption\"); ensures normal resource_return_none(); };";
    let c =
        load_resource_contract_sources(&BTreeMap::from([("g.qv".into(), text.into())])).unwrap();
    let mut metadata = premises(&c).unwrap();
    let parsed_graph: qcore_schemas::hypothesis::Graph =
        serde_json::from_value(metadata["$hypothesis_graphs"]["f"].clone()).unwrap();
    assert_eq!(
        metadata["$hypothesis_graph_digests"]["f"],
        qcore_contract::hypothesis::digest(&parsed_graph).unwrap()
    );
    assert_eq!(
        metadata["$hypothesis_graphs"]["f"]["atoms"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
    let units = vec![
        json!({"functions":[function("use_f","Safe",json!({"declarations":["f"],"hypothesis_dependencies":{"f":["legacy_00000000"]}}))]}),
    ];
    let rev = json!({"contracts":{"f":{"status":"pinned-in-force","mode":"pinned","contract_sha256":"pin","module":"g.qv"}}});
    let mut section = render(&Facts::of_rows(&units), &rev, &metadata);
    assert_eq!(
        section["declarations"]["f"]["hypothesis_evidence"]["disposition"],
        "open"
    );
    assert_eq!(
        section["declarations"]["f"]["body_standing"],
        "declared-assumption"
    );
    erase_hypotheses(&mut metadata);
    assert_eq!(
        metadata,
        json!({"f":[{"name":"p","statement":"assumption","origin":"declared"}]})
    );
    let mut old_units = json!(units);
    erase_hypotheses(&mut old_units);
    let expected = render(
        &Facts::of_rows(old_units.as_array().unwrap()),
        &rev,
        &metadata,
    );
    erase_hypotheses(&mut section);
    assert_eq!(section, expected);
}

#[test]
fn erasure_preserves_legacy_declarations_named_like_graph_fields() {
    let old = json!({"hypothesis_context":[{"name":"p","statement":"assumption","origin":"declared"}],"hypothesis_graph":{"mode":"pinned"},"hypothesis_dependencies":[]});
    let mut erased = old.clone();
    qcore_audit::contract_dependencies::erase_hypotheses(&mut erased);
    assert_eq!(erased, old);
}

#[test]
fn report_owned_cjson_atoms_follow_declaration_dependencies_without_affecting_unrelated_rows() {
    use qcore_c_gateway::contract_dependencies::{RowDependencies, hypothesis_closure};
    use std::collections::BTreeSet;
    let text = include_str!("../../../docs/reports/premise-decomposition-cycle1/proposed-cjson.qv");
    let contracts = qcore_contract::contract_loader::load_resource_contract_sources(
        &BTreeMap::from([("cjson.qv".into(), text.into())]),
    )
    .unwrap();
    let graphs = qcore_contract::hypothesis::replay(contracts.source_assumptions()).unwrap();
    let declaration = "cJSON_DetachItemViaPointer";
    let expected: BTreeSet<String> = graphs[declaration].atoms.keys().cloned().collect();
    assert_eq!(expected.len(), 10);
    for standing in ["direct", "summary", "held", "withdrawn"] {
        let mut deps = RowDependencies::default();
        match standing {
            "held" => {
                deps.held.insert(declaration.into());
            }
            "withdrawn" => {
                deps.withdrawn.insert(declaration.into());
            }
            _ => {
                deps.declarations.insert(declaration.into());
            }
        }
        if standing == "summary" {
            deps.summaries.insert("wrapper".into());
        }
        assert_eq!(hypothesis_closure(&deps, &graphs)[declaration], expected);
    }
    let unrelated = RowDependencies {
        declarations: BTreeSet::from(["cJSON_Delete".into()]),
        ..Default::default()
    };
    let closure = hypothesis_closure(&unrelated, &graphs);
    assert!(!closure.contains_key(declaration));
    assert!(
        !closure
            .values()
            .any(|atoms| atoms.contains("detach_no_writer"))
    );
    // Rendering keeps standing separate from graph disposition; no evidence control
    // above changes a verdict or validates the original representation premise.
    let metadata = premises(&contracts).unwrap();
    assert_eq!(metadata["$hypothesis_graphs"][declaration]["premises"]
        ["detach_representation_atoms"]["children"].as_array().unwrap().len(), 9);
}

#[test]
fn proposed_cjson_detach_consultation_propagates_atoms_through_a_summary() {
    use qcore_c_gateway::contract_dependencies::{Recording, hypothesis_closure, row_dependencies};
    use qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit;
    let production =
        include_str!("../../../docs/reports/premise-decomposition-cycle1/proposed-cjson.qv");
    let fixture = "module contract;
extern fn impact_alloc() -> ptr<mut,u8> contract { requires resource_contract_v1; ensures normal resource_return_owned(nonnull); };
extern fn impact_store(p: ptr<mut,u8>, i: ptr<mut,u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_owned(p,nonnull); ensures normal resource_stored(i,p,nonnull); ensures normal resource_return_none(); };
extern fn impact_free(p: ptr<mut,u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_released(p,nullable); ensures normal resource_return_none(); };";
    let contracts =
        qcore_contract::contract_loader::load_resource_contract_sources(&BTreeMap::from([
            ("cjson.qv".into(), production.into()),
            ("impact.qv".into(), fixture.into()),
        ]))
        .unwrap();
    let source = "void direct(void) { void *p=impact_alloc(); void *i=impact_alloc(); impact_store(p,i); void *r=cJSON_DetachItemViaPointer(p,i); impact_free(r); impact_free(p); } void caller(void) { direct(); } int unrelated(void) { return 7; }";
    let original =
        include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv");
    let actual =
        qcore_contract::contract_loader::load_resource_contract_sources(&BTreeMap::from([
            ("cjson.qv".into(), original.into()),
            ("impact.qv".into(), fixture.into()),
        ]))
        .unwrap();
    let actual_recording = Recording::start();
    let actual_summary = analyze_legacy_c_translation_unit(source, "caller", &actual).unwrap();
    let actual_record = actual_recording.finish();
    let actual_rows = row_dependencies(&actual_record, &actual_summary, &actual);
    let actual_graphs = qcore_contract::hypothesis::replay(actual.source_assumptions()).unwrap();
    // Resolve the exact production graph's formal references before the
    // context-scoped synthetic refutation/impact control.
    let manifest_bytes =
        include_bytes!("../../../docs/reports/premise-evidence-contracts/binding-manifest.json");
    let manifest_sha = qcore_schemas::sha256_hex(manifest_bytes);
    let expected_manifests = BTreeMap::from([(
        manifest_sha.clone(),
        serde_json::from_slice(manifest_bytes).unwrap(),
    )]);
    let presented_manifests = BTreeMap::from([(manifest_sha, manifest_bytes.to_vec())]);
    let artifacts = BTreeMap::from([
        (
            "proof/QCore/CjsonRepresentationPremises.lean".into(),
            include_bytes!("../../../proof/QCore/CjsonRepresentationPremises.lean").to_vec(),
        ),
        (
            "proof/AuditPremiseRefinement.lean".into(),
            include_bytes!("../../../proof/AuditPremiseRefinement.lean").to_vec(),
        ),
    ]);
    let resolved = qcore_contract::hypothesis_bindings::resolve(
        actual.source_assumptions(),
        &actual_graphs,
        &expected_manifests,
        &presented_manifests,
        &artifacts,
    )
    .unwrap();
    let bindings = resolved.values().next().unwrap();
    let bound_graph = &actual_graphs["cJSON_DetachItemViaPointer"];
    assert_eq!(
        bindings.graph_sha256,
        qcore_contract::hypothesis::digest(bound_graph).unwrap()
    );
    let held = qcore_schemas::hypothesis::Context {
        artifact: "fixture.c#direct".into(),
        property: "resource-custody.v1".into(),
        semantic_profile: "test-fixture-profile".into(),
        scope: "direct,caller,unrelated".into(),
        bounds: "finite synthetic fixture; source binding Open".into(),
        assumptions: "original declared representation premise".into(),
        source_identity: qcore_schemas::sha256_hex(source.as_bytes()),
        build_identity: "independently-held-test-build".into(),
        evidence_version: "premise-evidence-contracts-control.v1".into(),
    };
    let claim = qcore_schemas::hypothesis::Instance {
        graph: bound_graph.clone(),
        graph_sha256: bindings.graph_sha256.clone(),
        context: held.clone(),
        evidence: vec![qcore_schemas::hypothesis::Evidence {
            atom: "detach_no_writer".into(),
            atom_sha256: bindings.atoms["detach_no_writer"].atom_sha256.clone(),
            context: held.clone(),
            disposition: qcore_schemas::hypothesis::Disposition::Refuted,
            warrant: qcore_schemas::hypothesis::Warrant::Declared,
            reference:
                "synthetic model writer witness; source binding Open, no reachable C violation"
                    .into(),
        }],
    };
    assert_eq!(
        qcore_contract::hypothesis::compose_refinement(
            &claim,
            &held,
            bound_graph,
            "detach_list_representation"
        )
        .unwrap(),
        qcore_schemas::hypothesis::Disposition::Refuted
    );
    assert_eq!(
        qcore_contract::hypothesis::compose(&claim, &held, bound_graph, "legacy_premise_00000000")
            .unwrap(),
        qcore_schemas::hypothesis::Disposition::Open
    );
    let mut caller_context = held.clone();
    caller_context.artifact = "fixture.c#caller".into();
    assert!(
        qcore_contract::hypothesis::compose_refinement(
            &claim,
            &caller_context,
            bound_graph,
            "detach_list_representation"
        )
        .is_err()
    );
    for name in ["direct", "caller"] {
        assert_eq!(
            actual_summary.functions[name].status,
            qcore_c_gateway::legacy_c::AnalysisStatus::Safe
        );
        let closure = hypothesis_closure(&actual_rows[name], &actual_graphs);
        assert_eq!(closure["cJSON_DetachItemViaPointer"].len(), 10);
        assert!(closure["cJSON_DetachItemViaPointer"].contains("detach_no_writer"));
    }
    let recording = Recording::start();
    let summary = analyze_legacy_c_translation_unit(source, "caller", &contracts).unwrap();
    let record = recording.finish();
    let rows = row_dependencies(&record, &summary, &contracts);
    let graphs = qcore_contract::hypothesis::replay(contracts.source_assumptions()).unwrap();
    for name in ["direct", "caller"] {
        assert_eq!(
            summary.functions[name].status,
            qcore_c_gateway::legacy_c::AnalysisStatus::Safe,
            "{name}: {:?}",
            summary.functions[name]
        );
        let closure = hypothesis_closure(&rows[name], &graphs);
        assert!(
            closure["cJSON_DetachItemViaPointer"].contains("detach_no_writer"),
            "{name}: {:?}",
            rows[name]
        );
        assert_eq!(closure["cJSON_DetachItemViaPointer"].len(), 10);
    }
    assert!(
        rows["caller"].summaries.contains("direct"),
        "{:?}",
        rows["caller"]
    );
    assert!(hypothesis_closure(&rows["unrelated"], &graphs).is_empty());
}

#[test]
fn a_companion_premise_changes_closed_world_classification_so_stays_report_owned() {
    use qcore_audit::outer_selection::{Fact, custody_facts};
    use std::collections::BTreeSet;
    let original =
        include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv");
    let proposal =
        include_str!("../../../docs/reports/premise-decomposition-cycle1/proposed-cjson.qv");
    let load = |text: &str| {
        qcore_contract::contract_loader::load_resource_contract_sources(&BTreeMap::from([(
            "cjson.qv".into(),
            text.into(),
        )]))
        .unwrap()
    };
    let before = premises(&load(original)).unwrap();
    let after = premises(&load(proposal)).unwrap();
    let dependencies = json!({"declarations":["cJSON_DetachItemViaPointer"]});
    let old = custody_facts(&dependencies, &before);
    let new = custody_facts(&dependencies, &after);
    assert!(old.is_subset(&new));
    assert_eq!(
        new.difference(&old).cloned().collect::<BTreeSet<_>>(),
        BTreeSet::from([Fact::Premise {
            name: "detach_representation_atoms".into()
        }])
    );
    // This classification difference exists even when an Unknown verdict hides
    // it from the projection's moved-only records. No classifier exception is added.
}

#[test]
fn concrete_attachment_preserves_closed_world_facts_and_publishes_checked_open_bindings() {
    use qcore_audit::outer_selection::custody_facts;
    let current =
        include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv");
    let old = include_str!("../../../docs/reports/premise-decomposition-cycle1/proposed-cjson.qv")
        .lines()
        .filter(|line| {
            !line.contains("resource_hypothesis(detach_")
                && !line.contains("resource_premise_all(detach_representation_atoms,")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let load = |text: String| {
        qcore_contract::contract_loader::load_resource_contract_sources(&BTreeMap::from([(
            "cjson.qv".into(),
            text,
        )]))
        .unwrap()
    };
    let before = load(old);
    let after = load(current.into());
    assert_eq!(
        qcore_contract::contract_loader::declared_premises(&before).unwrap(),
        qcore_contract::contract_loader::declared_premises(&after).unwrap()
    );
    let old_meta = premises(&before).unwrap();
    let mut metadata = premises(&after).unwrap();
    let deps = json!({"declarations":["cJSON_DetachItemViaPointer"]});
    assert_eq!(
        custody_facts(&deps, &old_meta),
        custody_facts(&deps, &metadata)
    );
    assert_eq!(
        metadata["$hypothesis_binding_requirements"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let expected_graphs = qcore_contract::hypothesis::replay(after.source_assumptions()).unwrap();
    let bytes =
        include_bytes!("../../../docs/reports/premise-evidence-contracts/binding-manifest.json");
    let sha = qcore_schemas::sha256_hex(bytes);
    let expected = BTreeMap::from([(sha.clone(), serde_json::from_slice(bytes).unwrap())]);
    let presented = BTreeMap::from([(sha, bytes.to_vec())]);
    let artifacts = BTreeMap::from([
        (
            "proof/QCore/CjsonRepresentationPremises.lean".into(),
            include_bytes!("../../../proof/QCore/CjsonRepresentationPremises.lean").to_vec(),
        ),
        (
            "proof/AuditPremiseRefinement.lean".into(),
            include_bytes!("../../../proof/AuditPremiseRefinement.lean").to_vec(),
        ),
    ]);
    qcore_audit::contract_dependencies::publish_checked_bindings(
        &mut metadata,
        after.source_assumptions(),
        &expected_graphs,
        &expected,
        &presented,
        &artifacts,
    )
    .unwrap();
    assert_eq!(
        metadata["$checked_hypothesis_bindings"]["warrant"],
        "reference-integrity-only"
    );
    assert_eq!(
        metadata["$checked_hypothesis_bindings"]["source_binding"],
        "open"
    );
    let mut changed = metadata.clone();
    changed["$hypothesis_graphs"]["cJSON_DetachItemViaPointer"]["version"] = json!(1);
    assert!(
        qcore_audit::contract_dependencies::publish_checked_bindings(
            &mut changed,
            after.source_assumptions(),
            &expected_graphs,
            &expected,
            &presented,
            &artifacts
        )
        .is_err()
    );
    let units = vec![json!({"functions":[function("use_detach","Safe",deps.clone())]})];
    let revision = json!({"contracts":{"cJSON_DetachItemViaPointer":{"status":"pinned-in-force","mode":"pinned","contract_sha256":"independently-held-test-pin","module":"cjson.qv"}}});
    let mut rendered = render(&Facts::of_rows(&units), &revision, &metadata);
    assert_eq!(
        rendered["declarations"]["cJSON_DetachItemViaPointer"]["hypothesis_binding_requirements"],
        metadata["$hypothesis_binding_requirements"]
    );
    let sidecar = json!({"contract_dependencies":rendered.clone()});
    let view = qcore_audit::contract_dependencies::premise_view_from_sidecar(&sidecar).unwrap();
    assert_eq!(custody_facts(&deps, &view), custody_facts(&deps, &old_meta));
    qcore_audit::contract_dependencies::erase_hypotheses(&mut rendered);
    assert!(
        rendered["declarations"]["cJSON_DetachItemViaPointer"]
            .get("hypothesis_binding_requirements")
            .is_none()
    );
    let mut a = old_meta;
    let mut b = metadata;
    qcore_audit::contract_dependencies::erase_hypotheses(&mut a);
    qcore_audit::contract_dependencies::erase_hypotheses(&mut b);
    assert_eq!(a, b);
}

#[test]
fn actual_sidecar_premise_location_preserves_named_classification_and_missing_schema_refuses() {
    let dependency = json!({"declarations":["f"]});
    let named = json!({"f":[{"name":"detach_list_representation","statement":"exact original","origin":"declared"}]});
    let actual = json!({"contract_dependencies":{"declarations":{"f":{"premises":named["f"]},"no_premise":{}}}});
    let view = qcore_audit::contract_dependencies::premise_view_from_sidecar(&actual).unwrap();
    assert_eq!(view["f"], named["f"]);
    assert_eq!(view["no_premise"], json!([]));
    let expected = qcore_audit::outer_selection::custody_facts(&dependency, &named);
    assert!(!expected.is_empty());
    assert_eq!(
        qcore_audit::outer_selection::custody_facts(&dependency, &view),
        expected
    );
    assert_ne!(
        qcore_audit::outer_selection::custody_facts(&dependency, &actual["contract_premises"]),
        expected
    );
    assert!(qcore_audit::contract_dependencies::premise_view_from_sidecar(&json!({})).is_err());
    let mut malformed = actual;
    malformed["contract_dependencies"]["declarations"]["f"]["premises"] = json!("wrong");
    assert!(qcore_audit::contract_dependencies::premise_view_from_sidecar(&malformed).is_err());
}
