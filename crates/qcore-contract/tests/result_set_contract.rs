//! Card CELL-GUARD-RESULT-RANGE-CHECKED (contract specification C09.4,
//! grammar G05j): the C library's result sets load as flexible bounds
//! contract v1 declarations stating a value set only, every spelling of one
//! set has one canonical digest, and every malformed, empty, out-of-type or
//! custody-bearing form is refused.

use std::collections::BTreeMap;

use qcore_contract::bounds_contract::{BoundsClause, load_bounds_contract_sources};
use qcore_contract::contract_loader::load_resource_contract_sources;

const LIBC: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/result_sets.qv");

fn one(text: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("result_sets.qv".to_owned(), text.to_owned())])
}

fn declaration(clauses: &str, result: &str) -> String {
    format!(
        "module contract;\nextern fn f(x: i32) -> {result}\ncontract {{\n    requires bounds_contract_v1;\n{clauses}}};\n"
    )
}

fn flexible(set: &str) -> String {
    declaration(
        &format!(
            "    requires bounds_revision(flexible);\n    ensures normal result_set({set});\n"
        ),
        "i32",
    )
}

fn error_of(text: &str) -> String {
    load_bounds_contract_sources(&one(text)).expect_err(text)
}

#[test]
fn the_libc_result_sets_load_as_flexible_value_sets() {
    let contracts = load_bounds_contract_sources(&one(LIBC)).expect("loads");
    let canonical = |name: &str| {
        let contract = contracts.contract(name).expect(name);
        assert!(contract.flexible, "{name}");
        assert!(!contract.held(), "{name}");
        assert_eq!(contract.clauses.len(), 1, "{name}");
        contract.result_values().expect(name).canonical()
    };
    assert_eq!(canonical("pipe"), "[-1, 0]");
    assert_eq!(canonical("epoll_ctl"), "[-1, 0]");
    assert_eq!(canonical("open"), "[-1, MAX]");
    // Only the null property and the result-set derivation read it: the
    // bounds consumer's view drops every declaration of the module.
    assert!(contracts.for_bounds().functions.is_empty());
    assert!(contracts.for_bounds().sources.is_empty());
    // The resource loader rejects the module: no custody can come from it.
    assert!(load_resource_contract_sources(&one(LIBC)).is_err());
}

#[test]
fn every_spelling_of_one_set_has_one_canonical_digest() {
    let load = |set: &str| {
        let contracts = load_bounds_contract_sources(&one(&flexible(set))).expect(set);
        let f = contracts.contract("f").unwrap().clone();
        (f.canonical_sha256, f.text_sha256, f.clauses[0].text.clone())
    };
    let (a, ta, text) = load("-1 | [0, MAX]");
    let (b, tb, _) = load("[-1, MAX]");
    let (c, tc, _) = load("[MIN, MAX] ^ [MIN, -2]");
    let (d, _, _) = load("(-1 | [0, 5]) | [6, MAX]");
    assert_eq!(text, "result_set([-1, MAX])");
    assert_eq!(a, b);
    assert_eq!(a, c);
    assert_eq!(a, d);
    // The pin digest is over the text as written, as for every clause.
    assert_ne!(ta, tb);
    assert_ne!(tb, tc);
}

#[test]
fn the_operators_have_set_meaning_and_c_precedence() {
    let set = |text: &str| {
        load_bounds_contract_sources(&one(&flexible(text)))
            .expect(text)
            .contract("f")
            .unwrap()
            .result_values()
            .unwrap()
            .canonical()
    };
    assert_eq!(set("[0, 9] & [5, 12]"), "[5, 9]");
    assert_eq!(set("[0, 9] ^ [5, 12]"), "[0, 4] | [10, 12]");
    assert_eq!(set("-1 | [2, 3] & [3, 9]"), "-1 | 3");
    assert_eq!(set("(-1 | [2, 3]) & [3, 9]"), "3");
    assert_eq!(set("MIN | MAX"), "MIN | MAX");
}

#[test]
fn malformed_empty_and_out_of_type_sets_are_refused() {
    for (set, code) in [
        ("[0, 5] & [6, 9]", "result-set-empty"),
        ("0 ^ 0", "result-set-empty"),
        ("[5, 1]", "result-set-empty"),
        ("2147483648", "result-set-outside-type"),
        ("-2147483649", "result-set-outside-type"),
        ("1 || 2", "result-set-shape"),
        ("1 && 2", "result-set-shape"),
        ("[1, 2", "delimiter"),
        ("01", "result-set-shape"),
        ("-0", "result-set-shape"),
        ("x", "result-set-shape"),
        ("[| 1 |]", "result-set-shape"),
        ("", "result-set-shape"),
    ] {
        let error = error_of(&flexible(set));
        assert!(error.contains(code), "{set}: {error}");
    }
}

#[test]
fn a_result_set_stands_alone_on_a_flexible_integer_declaration() {
    // No flexible marker.
    let error = error_of(&declaration(
        "    ensures normal result_set(-1 | 0);\n",
        "i32",
    ));
    assert!(error.contains("result-set-needs-flexible"), "{error}");
    // The marker on anything else, or beside another clause.
    let error = error_of(&declaration(
        "    requires bounds_revision(flexible);\n    ensures normal nonnull_result_range(-1, 0);\n",
        "i32",
    ));
    assert!(error.contains("bounds-revision-flexible-scope"), "{error}");
    let error = error_of(&declaration(
        "    requires bounds_revision(flexible);\n    ensures normal result_set(-1 | 0);\n    ensures normal nonnull_result_range(-1, 0);\n",
        "i32",
    ));
    assert!(error.contains("bounds-revision-flexible-scope"), "{error}");
    let error = error_of(&declaration(
        "    requires bounds_revision(pinned);\n    ensures normal result_set(-1 | 0);\n",
        "i32",
    ));
    assert!(error.contains("bounds-revision-shape"), "{error}");
    // A precondition, or a pointer result.
    let error = error_of(&declaration(
        "    requires bounds_revision(flexible);\n    requires result_set(-1 | 0);\n",
        "i32",
    ));
    assert!(error.contains("result-set-ensures"), "{error}");
    let error = error_of(&declaration(
        "    requires bounds_revision(flexible);\n    ensures normal result_set(-1 | 0);\n",
        "ptr<mut, u8>",
    ));
    assert!(error.contains("result-set-integer"), "{error}");
    // A held result set is refused rather than read held.
    let error = error_of(&declaration(
        "    requires bounds_revision(flexible);\n    ensures normal bounds_review_pending(result_set(-1 | 0), \"why\");\n",
        "i32",
    ));
    assert!(!error.is_empty());
}

#[test]
fn the_universe_is_the_declared_result_type() {
    let usize_max = load_bounds_contract_sources(&one(&declaration(
        "    requires bounds_revision(flexible);\n    ensures normal result_set([1, MAX]);\n",
        "usize",
    )))
    .unwrap();
    let f = usize_max.contract("f").unwrap();
    assert_eq!(
        f.result_values().unwrap().intervals(),
        &[(1, i128::from(u64::MAX))]
    );
    assert!(matches!(
        f.clauses[0].clause,
        BoundsClause::ResultSet { .. }
    ));
    let error = error_of(&declaration(
        "    requires bounds_revision(flexible);\n    ensures normal result_set(-1);\n",
        "u32",
    ));
    assert!(error.contains("result-set-outside-type"), "{error}");
}
