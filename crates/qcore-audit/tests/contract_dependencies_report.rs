//! Card **VERDICT-CONTRACT-DEPENDENCIES** (rule 13h): the run's dependency
//! section and the shared `standingVectors` of
//! `proof/QCore/ContractDependencies.lean`.

use std::collections::BTreeMap;

use qcore_audit::contract_dependencies::{
    Facts, PREMISES_KEY, ROW_KEY, SECTION_KEY, UNIT_KEY, body_standing_of, premises, render,
};
use serde_json::{Value, json};

const LEAN: &str = include_str!("../../../proof/QCore/ContractDependencies.lean");

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
            UNIT_KEY: {"validated": {"v": ["W4"]}, "unvalidated": ["a"], "failed": {}, "withdrawn": {}},
        }),
        json!({
            "library": "l", "file": "b.c",
            "functions": [function("again", "Safe", json!({"declarations": ["v"]}))],
            UNIT_KEY: {"validated": {"v": ["entry"]}, "unvalidated": [], "failed": {}, "withdrawn": {}},
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
    assert_eq!(SECTION_KEY, "contract_dependencies");
    assert_eq!(PREMISES_KEY, "contract_premises");
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
