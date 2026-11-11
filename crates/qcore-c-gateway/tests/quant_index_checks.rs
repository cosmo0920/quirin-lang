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

//! Card QUANT-INDEX-CHECKS (gateway rule 12l): `quant checks P, Idx #i: …;`
//! through the route-G bounds consumer, on both sides of the call boundary.
//! The fixtures are modelled on chunkio's `cio_file_st_get_meta_len`, which
//! reads `map[22]` and `map[23]` of its header (test-only declarations; no
//! reviewed chunkio contract is written by this card).

#[path = "support/clang_gap.rs"]
mod clang_gap;

use std::collections::BTreeMap;

use clang_gap::*;
use qcore_c_gateway::index_extent::{BoundsFunctionRow, analyze_bounds_c11, row_verdict};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::bounds_contract::{BoundsContracts, load_bounds_contract_sources};
use qcore_core::index_extent::BoundsVerdict;

use BoundsVerdict::{Pending, Safe, Unknown, Unsafe};

fn contracts(text: &str) -> BoundsContracts {
    load_bounds_contract_sources(&BTreeMap::from([(
        "quant.qv".to_owned(),
        format!("module contract;\n{text}"),
    )]))
    .unwrap_or_else(|error| panic!("bounds fixture loads: {error}\n{text}"))
}

fn rows(source: &str, functions: &[&str], bounds: &BoundsContracts) -> Vec<BoundsFunctionRow> {
    let text = format!("{DECLARATIONS}\n{source}\n");
    analyze_bounds_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        &functions
            .iter()
            .map(|f| (*f).to_owned())
            .collect::<Vec<_>>(),
        bounds,
    )
    .unwrap_or_else(|error| panic!("bounds consumer refused the unit: {error}\n{source}"))
    .functions
}

fn row(source: &str, function: &str, bounds: &BoundsContracts) -> BoundsFunctionRow {
    rows(source, &[function], bounds).remove(0)
}

fn has_reason(row: &BoundsFunctionRow, fragment: &str) -> bool {
    row.reasons.iter().any(|r| r.contains(fragment))
}

/// `get_meta_len`'s declaration with the given predicate (and an optional
/// contract-level review marker).
fn meta(predicate: &str, held: bool) -> String {
    format!(
        "extern fn get_meta_len(map: ptr<mut, u8>) -> u16
contract {{
    requires bounds_contract_v1;
{}    quant checks map, Idx #i: {predicate};
}};
",
        if held {
            "    requires bounds_review_pending(\"chunkio header layout\");\n"
        } else {
            ""
        }
    )
}

const META_BODY: &str = "unsigned short get_meta_len(char *map) {
    return (unsigned short) (((unsigned char) map[22] << 8) | (unsigned char) map[23]);
}";

// ---------------------------------------------------------------------------
// Body side
// ---------------------------------------------------------------------------

#[test]
fn the_motivating_body_is_safe_within_its_check_and_pending_when_held() {
    if !clang_available() {
        return;
    }
    let r = row(
        META_BODY,
        "get_meta_len",
        &contracts(&meta("0 <= #i && #i < 24", false)),
    );
    assert_eq!(row_verdict(&r), Safe, "{r:#?}");
    assert!(
        r.scope
            .contains("quant checks map, Idx #i: 0 <= #i && #i < 24"),
        "{r:#?}"
    );
    assert!(!r.refutes_own_contract);
    let r = row(
        META_BODY,
        "get_meta_len",
        &contracts(&meta("0 <= #i && #i < 24", true)),
    );
    assert_eq!(row_verdict(&r), Pending, "{r:#?}");
    assert!(has_reason(&r, "awaiting-human-review"), "{r:#?}");
}

#[test]
fn every_comparison_operator_admits_exactly_its_indices() {
    if !clang_available() {
        return;
    }
    let cases = [
        ("#i >= 22 && #i <= 23", Safe),
        ("#i > 21 && #i < 24", Safe),
        ("22 <= #i && 23 >= #i", Safe),
        ("#i == 22 || #i == 23", Safe),
        ("(0 <= #i && #i < 4) || (22 <= #i && #i < 24)", Safe),
        // Off by one at the lower end, then at the upper end.
        ("#i >= 23 && #i < 24", Unknown),
        ("#i > 22 && #i <= 23", Unknown),
        ("#i >= 22 && #i < 23", Unknown),
        ("#i >= 22 && #i <= 22", Unknown),
        ("#i == 22", Unknown),
    ];
    for (predicate, expected) in cases {
        let r = row(
            META_BODY,
            "get_meta_len",
            &contracts(&meta(predicate, false)),
        );
        assert_eq!(row_verdict(&r), expected, "{predicate}: {r:#?}");
        if expected == Unknown {
            // The body accesses an index the check excludes: it refutes its
            // declaration, and never accuses.
            assert!(r.refutes_own_contract, "{predicate}: {r:#?}");
            assert!(has_reason(&r, "bounds-contract-refuted-by-body"), "{r:#?}");
            assert!(
                r.accesses
                    .iter()
                    .any(|a| a.actual.contains("bounds-quant-check-refuted")),
                "{r:#?}"
            );
        }
    }
}

#[test]
fn a_disjunction_is_read_literally_and_a_gap_index_refutes() {
    if !clang_available() {
        return;
    }
    // Index 5 lies in the hull of `[0, 3] ∪ [8, 11]` (so in bounds of the
    // extent the callers provide) but outside the check: the body refutes it.
    let c = contracts(&meta("(0 <= #i && #i < 4) || (8 <= #i && #i < 12)", false));
    let r = row(
        "unsigned short get_meta_len(char *map) { return map[3] + map[8]; }",
        "get_meta_len",
        &c,
    );
    assert_eq!(row_verdict(&r), Safe, "{r:#?}");
    let r = row(
        "unsigned short get_meta_len(char *map) { return map[3] + map[5]; }",
        "get_meta_len",
        &c,
    );
    assert_eq!(row_verdict(&r), Unknown, "{r:#?}");
    assert!(r.refutes_own_contract, "{r:#?}");
}

#[test]
fn an_undecided_index_is_never_safe() {
    if !clang_available() {
        return;
    }
    let text = "extern fn pick(map: ptr<mut, u8>, k: i32) -> u8
contract {
    requires bounds_contract_v1;
    quant checks map, Idx #i: 0 <= #i && #i < 24;
};
";
    let r = row(
        "unsigned char pick(char *map, int k) { return map[k]; }",
        "pick",
        &contracts(text),
    );
    assert_eq!(row_verdict(&r), Unknown, "{r:#?}");
    assert!(
        r.accesses
            .iter()
            .any(|a| a.actual.contains("bounds-quant-check-undecided")),
        "{r:#?}"
    );
    assert!(
        !r.refutes_own_contract,
        "undecided is not a refutation: {r:#?}"
    );
    // A counted loop over exactly the admitted range is decided inside.
    let r = row(
        "unsigned char pick(char *map, int k) { int s = 0; for (int i = 0; i < 24; i++) s += map[i]; return s; }",
        "pick",
        &contracts(text),
    );
    assert_eq!(row_verdict(&r), Safe, "{r:#?}");
    // One more iteration: a range not contained, never a refutation.
    let r = row(
        "unsigned char pick(char *map, int k) { int s = 0; for (int i = 0; i < 25; i++) s += map[i]; return s; }",
        "pick",
        &contracts(text),
    );
    assert_eq!(row_verdict(&r), Unknown, "{r:#?}");
    assert!(!r.refutes_own_contract, "{r:#?}");
}

#[test]
fn dereference_member_and_copies_are_accesses_through_the_formal() {
    if !clang_available() {
        return;
    }
    let text = "extern fn walk(map: ptr<mut, u8>) -> u8
contract {
    requires bounds_contract_v1;
    quant checks map, Idx #i: 22 <= #i && #i <= 23;
};
";
    // `*map` is index 0, outside `[22, 23]`.
    let r = row(
        "unsigned char walk(char *map) { return *map; }",
        "walk",
        &contracts(text),
    );
    assert_eq!(row_verdict(&r), Unknown, "{r:#?}");
    assert!(r.refutes_own_contract, "{r:#?}");
    // A copy at an offset keeps the object: `p[1]` is `map[23]`.
    let r = row(
        "unsigned char walk(char *map) { char *p = map + 22; return p[1]; }",
        "walk",
        &contracts(text),
    );
    assert_eq!(row_verdict(&r), Safe, "{r:#?}");
    let r = row(
        "unsigned char walk(char *map) { char *p = map + 22; return p[2]; }",
        "walk",
        &contracts(text),
    );
    assert_eq!(row_verdict(&r), Unknown, "{r:#?}");
    assert!(r.refutes_own_contract, "{r:#?}");
    // A conversion that changes the element type leaves no fact.
    let r = row(
        "unsigned char walk(char *map) { int *q = (int *) map; return q[0] != 0; }",
        "walk",
        &contracts(text),
    );
    assert_eq!(row_verdict(&r), Unknown, "{r:#?}");
}

#[test]
fn the_seeded_extent_is_a_lower_bound_and_never_accuses() {
    if !clang_available() {
        return;
    }
    // Forming `map + 30` is beyond the extent the check requires of callers,
    // but the object may be larger: never a validated violation.
    let r = row(
        "unsigned short get_meta_len(char *map) { char *end = map + 30; (void) end; return map[22]; }",
        "get_meta_len",
        &contracts(&meta("0 <= #i && #i < 24", false)),
    );
    assert_eq!(row_verdict(&r), Unknown, "{r:#?}");
    assert!(has_reason(&r, "bounds-range-not-contained"), "{r:#?}");
}

#[test]
fn a_formal_relative_check_reads_like_a_declared_extent() {
    if !clang_available() {
        return;
    }
    let check = |predicate: &str| {
        contracts(&format!(
            "extern fn sum(buf: ptr<mut, u8>, len: i32) -> i32
contract {{
    requires bounds_contract_v1;
    quant checks buf, Idx #i: {predicate};
}};
"
        ))
    };
    let body = "int sum(char *buf, int len) { int s = 0; for (int i = 0; i < len; i++) s += buf[i]; return s; }";
    assert_eq!(
        row_verdict(&row(body, "sum", &check("0 <= #i && #i < len"))),
        Safe
    );
    assert_eq!(
        row_verdict(&row(body, "sum", &check("#i >= 0 && #i <= len - 1"))),
        Safe
    );
    // A formal plus an offset is accepted by the loader but has no fact.
    let r = row(body, "sum", &check("0 <= #i && #i < len + 1"));
    assert_eq!(row_verdict(&r), Unknown, "{r:#?}");
    assert!(has_reason(&r, "bounds-quant-check-no-fact"), "{r:#?}");
    // A written formal withdraws the symbolic fact.
    let r = row(
        "int sum(char *buf, int len) { len = len + 1; return buf[0]; }",
        "sum",
        &check("0 <= #i && #i < len"),
    );
    assert_eq!(row_verdict(&r), Unknown, "{r:#?}");
}

// ---------------------------------------------------------------------------
// Call side
// ---------------------------------------------------------------------------

#[test]
fn a_caller_covers_every_admitted_index() {
    if !clang_available() {
        return;
    }
    let c = contracts(&meta("0 <= #i && #i < 24", false));
    let callee = "unsigned short get_meta_len(char *map);";
    let verdict = |body: &str| {
        let r = row(&format!("{callee}\n{body}"), "caller", &c);
        (row_verdict(&r), r)
    };
    let (v, r) = verdict("unsigned short caller(void) { char hdr[24]; return get_meta_len(hdr); }");
    assert_eq!(v, Safe, "{r:#?}");
    let (v, r) =
        verdict("unsigned short caller(void) { char hdr[64]; return get_meta_len(hdr + 40); }");
    assert_eq!(v, Safe, "{r:#?}");
    // Too small, definitely reached: a validated call-precondition violation.
    let (v, r) = verdict("unsigned short caller(void) { char hdr[16]; return get_meta_len(hdr); }");
    assert_eq!(v, Unsafe, "{r:#?}");
    let (v, r) =
        verdict("unsigned short caller(void) { char hdr[64]; return get_meta_len(hdr + 41); }");
    assert_eq!(v, Unsafe, "{r:#?}");
    // Too small under a branch: reachability unresolved, Unknown.
    let (v, r) = verdict(
        "unsigned short caller(void) { char hdr[16]; if (opaque()) return get_meta_len(hdr); return 0; }",
    );
    assert_eq!(v, Unknown, "{r:#?}");
    assert!(
        has_reason(&r, "bounds-violation-reachability-unresolved"),
        "{r:#?}"
    );
    // An actual with no established extent decides nothing.
    let (v, r) = verdict("unsigned short caller(char *p) { return get_meta_len(p); }");
    assert_eq!(v, Unknown, "{r:#?}");
    assert!(has_reason(&r, "bounds-missing-extent"), "{r:#?}");
    // A held callee: the obligation rests on its review.
    let held = contracts(&meta("0 <= #i && #i < 24", true));
    let r = row(
        &format!(
            "{callee}\nunsigned short caller(void) {{ char hdr[24]; return get_meta_len(hdr); }}"
        ),
        "caller",
        &held,
    );
    assert_eq!(row_verdict(&r), Pending, "{r:#?}");
}

#[test]
fn a_negative_lower_bound_needs_room_before_the_actual() {
    if !clang_available() {
        return;
    }
    let c = contracts(
        "extern fn around(p: ptr<mut, u8>) -> i32
contract {
    requires bounds_contract_v1;
    quant checks p, Idx #i: -2 <= #i && #i <= 1;
};
",
    );
    let r = row("int around(int *p) { return p[-2] + p[1]; }", "around", &c);
    assert_eq!(row_verdict(&r), Safe, "{r:#?}");
    let source = "int around(int *p);
int ok(void) { int a[4] = {0}; return around(a + 2); }
int bad(void) { int a[4] = {0}; return around(a); }";
    assert_eq!(row_verdict(&row(source, "ok", &c)), Safe);
    assert_eq!(row_verdict(&row(source, "bad", &c)), Unsafe);
}

#[test]
fn a_formal_relative_obligation_uses_the_actuals() {
    if !clang_available() {
        return;
    }
    let c = contracts(
        "extern fn fill(buf: ptr<mut, u8>, len: i32) -> unit
contract {
    requires bounds_contract_v1;
    quant checks buf, Idx #i: 0 <= #i && #i < len;
};
extern fn relay(p: ptr<mut, u8>, n: i32) -> unit
contract {
    requires bounds_contract_v1;
    quant checks p, Idx #i: 0 <= #i && #i < n;
};
",
    );
    let source = "void fill(char *buf, int len);
void exact(void) { char b[8]; fill(b, 8); }
void over(void) { char b[8]; fill(b, 9); }
void none(void) { char b[8]; fill(b, 0); }
void relay(char *p, int n) { fill(p, n); }";
    assert_eq!(row_verdict(&row(source, "exact", &c)), Safe);
    assert_eq!(row_verdict(&row(source, "over", &c)), Unsafe);
    // An empty admitted set under the actuals discharges nothing.
    let r = row(source, "none", &c);
    assert_eq!(row_verdict(&r), Unknown, "{r:#?}");
    assert!(has_reason(&r, "bounds-inconsistent-premises"), "{r:#?}");
    // The caller's own symbolic check covers the callee's: `[0, n)` into
    // `[0, n)`. Its pass-through stays undecided (the callee's set is not
    // constant), so the row is Unknown with a named escape.
    let r = row(source, "relay", &c);
    assert!(
        r.accesses
            .iter()
            .any(|a| a.kind == "call-precondition" && a.actual == "in-bounds"),
        "{r:#?}"
    );
    assert!(
        r.accesses
            .iter()
            .any(|a| a.actual.contains("bounds-quant-check-escapes")),
        "{r:#?}"
    );
}

#[test]
fn a_refuted_check_is_withdrawn_from_its_callers() {
    if !clang_available() {
        return;
    }
    let c = contracts(&meta("0 <= #i && #i < 24", false));
    let source = "unsigned short get_meta_len(char *map) { return map[24]; }
unsigned short caller(void) { char hdr[32]; return get_meta_len(hdr); }";
    let rs = rows(source, &["get_meta_len", "caller"], &c);
    assert_eq!(row_verdict(&rs[0]), Unknown, "{:#?}", rs[0]);
    assert!(rs[0].refutes_own_contract);
    assert_eq!(row_verdict(&rs[1]), Unknown, "{:#?}", rs[1]);
    assert!(
        has_reason(&rs[1], "states no bounds contract"),
        "{:#?}",
        rs[1]
    );
}

#[test]
fn passing_the_checked_formal_on_is_decided_by_the_callees_check() {
    if !clang_available() {
        return;
    }
    let c = contracts(&format!(
        "{}extern fn tail(m: ptr<mut, u8>) -> u8
contract {{
    requires bounds_contract_v1;
    quant checks m, Idx #i: 0 <= #i && #i <= 1;
}};
extern fn wide(m: ptr<mut, u8>) -> u8
contract {{
    requires bounds_contract_v1;
    quant checks m, Idx #i: 0 <= #i && #i < 30;
}};
",
        meta("0 <= #i && #i < 24", false)
    ));
    let decls =
        "unsigned char tail(char *m); unsigned char wide(char *m); void q_observe(void *p);";
    let r = row(
        &format!("{decls}\nunsigned short get_meta_len(char *map) {{ return tail(map + 22); }}"),
        "get_meta_len",
        &c,
    );
    assert_eq!(row_verdict(&r), Safe, "{r:#?}");
    for body in [
        "return wide(map);",
        "return tail(map + 23);",
        "q_observe(map); return 0;",
    ] {
        let r = row(
            &format!("{decls}\nunsigned short get_meta_len(char *map) {{ {body} }}"),
            "get_meta_len",
            &c,
        );
        assert_eq!(row_verdict(&r), Unknown, "{body}: {r:#?}");
        assert!(!r.refutes_own_contract, "an escape never refutes: {r:#?}");
    }
}
