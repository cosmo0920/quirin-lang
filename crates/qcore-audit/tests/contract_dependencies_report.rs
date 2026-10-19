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
