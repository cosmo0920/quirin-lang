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

//! Card CG16-INDEX-AND-OFFSET-EXTENTS (gateway rule 11s): the separately
//! selected bounds property through the route-G source consumer, against the
//! acceptance controls of `docs/plans/clang-gap-cards/cg16-reviewed-index-contracts.md`
//! §4. Every custody verdict asserted here is the custody gateway's own,
//! unchanged by the bounds consumer (CG00's controls stay custody limits).

#[path = "support/clang_gap.rs"]
mod clang_gap;

use std::collections::BTreeMap;

use clang_gap::*;
use qcore_c_gateway::index_extent::{
    BoundsFunctionRow, BoundsUnitReport, analyze_bounds_c11, check_bounds_report, row_verdict,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::bounds_contract::{BoundsContracts, load_bounds_contract_sources};
use qcore_core::index_extent::{BoundsVerdict, Request, compose};

const S: &str = "struct S { int a; int b; unsigned bf : 3; void *p; void *v; \
struct I { void *p; int x; } in; int arr[4]; void *slots[4]; void (*cb)(void); };\n";

fn none() -> BoundsContracts {
    BoundsContracts::default()
}

fn contracts(text: &str) -> BoundsContracts {
    load_bounds_contract_sources(&BTreeMap::from([(
        "cg16.qv".to_owned(),
        format!("module contract;\n{text}"),
    )]))
    .expect("bounds fixture loads")
}

fn text(source: &str) -> String {
    format!("{DECLARATIONS}\n{S}{source}\n")
}

fn report(source: &str, bounds: &BoundsContracts) -> BoundsUnitReport {
    let text = text(source);
    analyze_bounds_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        &["subject".to_owned()],
        bounds,
    )
    .unwrap_or_else(|error| panic!("bounds consumer refused the unit: {error}\n{source}"))
}

fn row(source: &str, bounds: &BoundsContracts) -> BoundsFunctionRow {
    report(source, bounds).functions.remove(0)
}

fn verdict(source: &str, bounds: &BoundsContracts) -> BoundsVerdict {
    row_verdict(&row(source, bounds))
}

fn custody(source: &str) -> BoundsVerdict {
    let run = g(&format!("{S}{source}"));
    if run.safe() {
        BoundsVerdict::Safe
    } else if run.unsafe_() {
        BoundsVerdict::Unsafe
    } else {
        BoundsVerdict::Unknown
    }
}

fn has_reason(row: &BoundsFunctionRow, fragment: &str) -> bool {
    row.reasons.iter().any(|r| r.contains(fragment))
}

use BoundsVerdict::{Pending, Safe, Unknown, Unsafe};

// ---------------------------------------------------------------------------
// The flagship and the fixed-array controls
// ---------------------------------------------------------------------------

/// `int arr[4]; arr[7]`: custody-only Safe as before, bounds a validated
/// violation with no user contract.
#[test]
fn cg16_flagship_out_of_bounds_access_is_a_bounds_violation_and_custody_is_unchanged() {
    if !clang_available() {
        return;
    }
    let source = "int subject(void) { int arr[4]; arr[0] = 1; return arr[7]; }";
    assert_eq!(custody(source), Safe, "the custody result is unchanged");
    let r = row(source, &none());
    assert_eq!(r.verdict, "Unsafe", "{r:#?}");
    assert!(
        r.accesses
            .iter()
            .any(|a| a.kind == "element-access" && a.actual == "violation")
    );
    assert_eq!(r.scope, "unconditional");
    // CG00's M14 control, the embedded array: custody Safe, bounds Unsafe
    // (the subobject has 4 elements whatever `s` points to).
    let m14 = "int subject(struct S *s) { return s->arr[7]; }";
    assert_eq!(custody(m14), Safe);
    let r = row(m14, &none());
    assert_eq!(r.verdict, "Unsafe", "{r:#?}");
    assert!(
        r.accesses
            .iter()
            .any(|a| a.kind == "member-access" && a.actual.starts_with("bounds-missing-extent"))
    );
}

#[test]
fn cg16_fixed_array_accesses_at_the_ends() {
    if !clang_available() {
        return;
    }
    let safe = "int subject(void) { int arr[4]; arr[0] = 1; arr[3] = 2; return arr[0] + arr[3]; }";
    assert_eq!(verdict(safe, &none()), Safe);
    for index in ["4", "7", "-1"] {
        let source = format!("int subject(void) {{ int arr[4]; return arr[{index}]; }}");
        assert_eq!(verdict(&source, &none()), Unsafe, "arr[{index}]");
    }
    // An enumeration constant and checked constant arithmetic in the C type.
    assert_eq!(
        verdict(
            "enum { N = 4 }; int subject(void) { int arr[N]; return arr[N - 1]; }",
            &none()
        ),
        Safe
    );
    assert_eq!(
        verdict(
            "enum { N = 4 }; int subject(void) { int arr[N]; return arr[N * 2]; }",
            &none()
        ),
        Unsafe
    );
    // Nested arrays: each dimension is its own extent.
    assert_eq!(
        verdict(
            "int subject(void) { int m[2][3]; return m[1][2]; }",
            &none()
        ),
        Safe
    );
    assert_eq!(
        verdict(
            "int subject(void) { int m[2][3]; return m[1][3]; }",
            &none()
        ),
        Unsafe
    );
    // A string literal's extent counts its terminator.
    assert_eq!(
        verdict("int subject(void) { return \"abc\"[3]; }", &none()),
        Safe
    );
    assert_eq!(
        verdict("int subject(void) { return \"abc\"[4]; }", &none()),
        Unsafe
    );
    // `sizeof` does not evaluate its operand.
    assert_eq!(
        verdict(
            "int subject(void) { int arr[4]; return sizeof(arr[9]); }",
            &none()
        ),
        Safe
    );
}

#[test]
fn cg16_one_past_formation_is_not_an_access() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        verdict(
            "int subject(void) { int arr[4]; int *e = arr + 4; return e == arr; }",
            &none()
        ),
        Safe
    );
    assert_eq!(
        verdict(
            "int subject(void) { int arr[4]; int *e = arr + 4; return *e; }",
            &none()
        ),
        Unsafe
    );
    assert_eq!(
        verdict(
            "int subject(void) { int arr[4]; int *e = &arr[4]; return e != 0; }",
            &none()
        ),
        Safe
    );
    assert_eq!(
        verdict(
            "int subject(void) { int arr[4]; int *e = arr + 5; return e != 0; }",
            &none()
        ),
        Unsafe
    );
    // CG00's M17 control: formed from an embedded array (custody IND).
    let m17 = "int subject(struct S *s) { int *e = s->arr + 4; return e == s->arr; }";
    assert!(g(&format!("{S}{m17}")).refused(IND));
    assert_eq!(verdict(m17, &none()), Unknown, "the base `s` has no extent");
}

#[test]
fn cg16_same_object_obligations_of_subtraction_and_ordering() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        verdict(
            "long subject(void) { int a[4]; return &a[3] - &a[0]; }",
            &none()
        ),
        Safe
    );
    assert_eq!(
        verdict(
            "long subject(void) { int a[4]; int b[4]; return &a[1] - &b[0]; }",
            &none()
        ),
        Unsafe,
        "two distinct named objects"
    );
    // CG00's M16 controls: custody Safe, bounds Unknown (no identity).
    for m16 in [
        "long subject(char *a, char *b) { return a - b; }",
        "int subject(char *a, char *b) { return a < b; }",
    ] {
        assert_eq!(custody(m16), Safe);
        let r = row(m16, &none());
        assert_eq!(r.verdict, "Unknown");
        assert!(r.accesses.iter().any(|a| a.kind == "same-object"));
    }
    // CG00's M15 controls: pointer arithmetic on an unknown extent.
    for m15 in [
        "int subject(int *p, int i) { return *(p + i); }",
        "void subject(void) { char *p = q_alloc(); char *q = p + 3; q_free(p); }",
    ] {
        assert_eq!(custody(m15), Safe);
        assert_eq!(verdict(m15, &none()), Unknown);
    }
}

// ---------------------------------------------------------------------------
// Values, guards, invalidation
// ---------------------------------------------------------------------------

#[test]
fn cg16_unsupported_ranges_and_missing_extents_are_unknown() {
    if !clang_available() {
        return;
    }
    let r = row("int subject(int i) { int arr[4]; return arr[i]; }", &none());
    assert_eq!(r.verdict, "Unknown");
    assert!(has_reason(&r, "bounds-unsupported-range"), "{r:#?}");
    let r = row("int subject(int *p) { return *p; }", &none());
    assert_eq!(r.verdict, "Unknown");
    assert!(has_reason(&r, "bounds-missing-extent"), "{r:#?}");
    // CG00's M14 computed index stays custody Safe; bounds Unknown.
    let m14 = "int subject(struct S *s, int i) { return s->arr[i]; }";
    assert_eq!(custody(m14), Safe);
    assert_eq!(verdict(m14, &none()), Unknown);
}

#[test]
fn cg16_guards_and_counted_loops_establish_ranges() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        verdict(
            "int subject(int i) { int arr[4]; if (i >= 0 && i < 4) return arr[i]; return 0; }",
            &none()
        ),
        Safe
    );
    assert_eq!(
        verdict(
            "int subject(unsigned i) { int arr[4]; if (i < 4) return arr[i]; return 0; }",
            &none()
        ),
        Safe
    );
    // A signed index with no lower bound is not in range.
    assert_eq!(
        verdict(
            "int subject(int i) { int arr[4]; if (i < 4) return arr[i]; return 0; }",
            &none()
        ),
        Unknown
    );
    // The negated guard on the else arm.
    assert_eq!(
        verdict(
            "int subject(unsigned i) { int arr[4]; if (i >= 4) return 0; else return arr[i]; }",
            &none()
        ),
        Safe
    );
    assert_eq!(
        verdict(
            "void subject(void) { int arr[4]; int i; for (i = 0; i < 4; i++) arr[i] = 0; }",
            &none()
        ),
        Safe
    );
    // A range wider than the extent is not a validated violation.
    assert_eq!(
        verdict(
            "void subject(void) { int arr[4]; int i; for (i = 0; i < 5; i++) arr[i] = 0; }",
            &none()
        ),
        Unknown
    );
    // A loop that writes its counter in the body is not canonical.
    assert_eq!(
        verdict(
            "void subject(void) { int arr[4]; int i; for (i = 0; i < 4; i++) { arr[i] = 0; i++; } }",
            &none()
        ),
        Unknown
    );
}

/// A guard establishes a range, then the index (or base) changes: the fact is
/// withdrawn or re-derived, never reused stale.
#[test]
fn cg16_a_changed_index_or_base_invalidates_the_fact() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        verdict(
            "int subject(unsigned i, unsigned j) { int arr[4]; if (i < 4) { i = j; return arr[i]; } return 0; }",
            &none()
        ),
        Unknown
    );
    // An address taken: never tracked, whatever the guard.
    assert_eq!(
        verdict(
            "void cg_take(unsigned *p); int subject(unsigned i) { int arr[4]; if (i < 4) { cg_take(&i); return arr[i]; } return 0; }",
            &none()
        ),
        Unknown
    );
    // A re-derived constant is used again.
    assert_eq!(
        verdict(
            "int subject(void) { int arr[4]; int i = 7; i = 2; return arr[i]; }",
            &none()
        ),
        Safe
    );
    assert_eq!(
        verdict(
            "int subject(void) { int arr[4]; int i = 2; i++; i++; return arr[i]; }",
            &none()
        ),
        Unsafe
    );
    // The base changes.
    assert_eq!(
        verdict(
            "int subject(int *q) { int arr[4]; int *p = arr; p = q; return p[1]; }",
            &none()
        ),
        Unknown
    );
    // A step of a known pointer is formed and tracked.
    assert_eq!(
        verdict(
            "int subject(void) { int arr[4]; int *p = arr; p++; p++; return p[1]; }",
            &none()
        ),
        Safe
    );
    assert_eq!(
        verdict(
            "int subject(void) { int arr[4]; int *p = arr; p += 3; return p[1]; }",
            &none()
        ),
        Unsafe
    );
}

#[test]
fn cg16_conversions_and_arithmetic_are_checked() {
    if !clang_available() {
        return;
    }
    // A defined conversion to unsigned: an exact, out-of-range index.
    assert_eq!(
        verdict(
            "int subject(void) { int arr[4]; return arr[(unsigned)-1]; }",
            &none()
        ),
        Unsafe
    );
    // An implementation-defined conversion refuses (no fabricated value).
    let r = row(
        "int subject(void) { int arr[4]; return arr[(signed char)200]; }",
        &none(),
    );
    assert_eq!(r.verdict, "Unknown");
    // Signed overflow in the index arithmetic refuses.
    let r = row(
        "int subject(void) { int arr[4]; return arr[2147483647 + 1]; }",
        &none(),
    );
    assert_eq!(r.verdict, "Unknown");
}

// ---------------------------------------------------------------------------
// Reachability
// ---------------------------------------------------------------------------

#[test]
fn cg16_reachability_is_validated_before_unsafe() {
    if !clang_available() {
        return;
    }
    // Proved unreachable: excluded by the explicit check, still inventoried.
    let r = row(
        "int subject(void) { int arr[4]; if (0) arr[7] = 1; return 0; }",
        &none(),
    );
    assert_eq!(r.verdict, "Safe", "{r:#?}");
    assert!(r.accesses.iter().any(|a| a.actual.starts_with("excluded")));
    let r = row(
        "int subject(void) { int arr[4]; return 0; arr[7] = 1; }",
        &none(),
    );
    assert_eq!(r.verdict, "Safe");
    assert!(r.accesses.iter().any(|a| a.actual.starts_with("excluded")));
    // Unresolved reachability: never an unvalidated Unsafe.
    let r = row(
        "int subject(int x) { int arr[4]; if (x) arr[7] = 1; return 0; }",
        &none(),
    );
    assert_eq!(r.verdict, "Unknown");
    assert!(has_reason(&r, "bounds-violation-reachability-unresolved"));
    // After a call (it may not return) and beside a call.
    assert_eq!(
        verdict(
            "int subject(void) { int arr[4]; cg_sink(1); return arr[7]; }",
            &none()
        ),
        Unknown
    );
    assert_eq!(
        verdict(
            "int subject(void) { int arr[4]; return opaque() + arr[7]; }",
            &none()
        ),
        Unknown
    );
    // Under a short-circuit operand.
    assert_eq!(
        verdict(
            "int subject(int x) { int arr[4]; return x && arr[7]; }",
            &none()
        ),
        Unknown
    );
}

/// Accesses in every expression position are inventoried, pointer reads and
/// writes included, without choosing an evaluation order.
#[test]
fn cg16_every_position_is_inventoried() {
    if !clang_available() {
        return;
    }
    let r = row(
        "void cg_use(int v); int subject(void) { int arr[4]; int i; int *p = arr; int x = arr[1]; \
         if (arr[1]) cg_use(arr[2]); while (arr[0]) { arr[3] = 0; } \
         for (i = 0; i < 4; i++) *p = arr[i]; return arr[3]; }",
        &none(),
    );
    let positions = r
        .accesses
        .iter()
        .map(|a| a.position.as_str())
        .collect::<Vec<_>>();
    for wanted in [
        "condition",
        "actual",
        "loop-body statement",
        "return",
        "declaration",
    ] {
        assert!(
            positions.iter().any(|p| p.contains(wanted)),
            "{wanted}: {positions:?}"
        );
    }
    assert!(r.accesses.iter().any(|a| a.kind == "dereference"));
    // The pointer actual of an uncontracted callee is a call obligation.
    assert!(r.accesses.iter().all(|a| a.kind != "call-precondition"));
    let r = row(
        "int subject(void) { int arr[4]; q_observe(arr); return 0; }",
        &none(),
    );
    assert_eq!(r.verdict, "Unknown");
    assert!(r.accesses.iter().any(|a| a.kind == "call-precondition"));
}

// ---------------------------------------------------------------------------
// Reviewed declarations
// ---------------------------------------------------------------------------

const RELATION: &str = "extern fn subject(p: ptr<mut, u8>, n: usize, i: usize) -> i32
contract {
    requires bounds_contract_v1;
    requires bounds_extent(p, n, elements);
    requires bounds_index(i, n);
};";

const RELATION_C: &str = "int subject(int *p, unsigned long n, unsigned long i) { return p[i]; }";

fn held(text: &str) -> String {
    text.replace(
        "requires bounds_contract_v1;",
        "requires bounds_contract_v1;\n    requires bounds_review_pending(\"is p really n elements long?\", \"cg16-owner\");",
    )
}

#[test]
fn cg16_a_held_relation_is_pending_and_resolution_is_conditional_safe() {
    if !clang_available() {
        return;
    }
    let held_contracts = contracts(&held(RELATION));
    let r = row(RELATION_C, &held_contracts);
    assert_eq!(r.verdict, "Pending", "{r:#?}");
    let reason = r
        .reasons
        .iter()
        .find(|x| x.contains("awaiting-human-review"))
        .expect("hold reason");
    for part in [
        "is p really n elements long?",
        "cg16-owner",
        "acceptable-as-written-after-review",
        "sha256",
    ] {
        assert!(reason.contains(part), "{part}: {reason}");
    }
    assert!(r.scope.contains("held"));
    // Resolved by editing the marker away: Safe, conditional on the
    // preconditions, never unconditional.
    let r = row(RELATION_C, &contracts(RELATION));
    assert_eq!(r.verdict, "Safe", "{r:#?}");
    assert!(r.scope.starts_with("conditional on"), "{}", r.scope);
    assert!(r.assumptions.iter().any(|a| a.starts_with("declared:")));
}

#[test]
fn cg16_approval_cannot_hide_another_obstacle() {
    if !clang_available() {
        return;
    }
    let source =
        "int subject(int *p, unsigned long n, unsigned long i, int *q) { return p[i] + *q; }";
    let text = held(RELATION).replace("i: usize)", "i: usize, q: ptr<mut, u8>)");
    let r = row(source, &contracts(&text));
    assert_eq!(r.verdict, "Unknown", "{r:#?}");
    assert!(has_reason(&r, "awaiting-human-review"));
    assert!(has_reason(&r, "bounds-missing-extent"));
}

#[test]
fn cg16_a_violation_resting_on_a_held_declaration_accuses_nothing() {
    if !clang_available() {
        return;
    }
    let text = "extern fn subject(p: ptr<mut, u8>) -> i32
contract {
    requires bounds_contract_v1;
    requires bounds_extent(p, 2, elements);
};";
    let source = "int subject(int *p) { return p[3]; }";
    let r = row(source, &contracts(&held(text)));
    assert_eq!(r.verdict, "Unknown", "{r:#?}");
    assert!(has_reason(&r, "bounds-held-premise-violation"));
    // In force (resolved), the same violation is validated.
    assert_eq!(verdict(source, &contracts(text)), Unsafe);
}

#[test]
fn cg16_a_declared_extent_never_enlarges_a_source_object() {
    if !clang_available() {
        return;
    }
    let claim = "extern fn subject() -> i32
contract {
    requires bounds_contract_v1;
    requires bounds_global_extent(table, 8, elements);
};";
    // The source states 4: the claim of 8 is a reported contradiction and
    // the source-supported violation remains.
    let r = row(
        "int table[4]; int subject(void) { return table[7]; }",
        &contracts(claim),
    );
    assert_eq!(r.verdict, "Unsafe", "{r:#?}");
    assert!(has_reason(&r, "bounds-declaration-contradicted"));
    let r = row(
        "int table[4]; int subject(void) { return table[2]; }",
        &contracts(claim),
    );
    assert_eq!(
        r.verdict, "Unknown",
        "a contradicted premise discharges nothing: {r:#?}"
    );
    // External storage the source does not size: the declaration supplies it.
    let external = "extern int table[]; int subject(void) { return table[7]; }";
    assert_eq!(verdict(external, &none()), Unknown);
    assert_eq!(verdict(external, &contracts(claim)), Safe);
    assert_eq!(verdict(external, &contracts(&held(claim))), Pending);
    // An empty declared extent admits no access.
    let empty = claim.replace("table, 8,", "table, 0,");
    assert_eq!(
        verdict(
            "extern int table[]; int subject(void) { return table[0]; }",
            &contracts(&empty)
        ),
        Unsafe
    );
}

// ---------------------------------------------------------------------------
// Call sites
// ---------------------------------------------------------------------------

const PICK: &str = "extern fn pick(i: i32) -> i32
contract {
    requires bounds_contract_v1;
    requires bounds_index(i, 4);
};";

#[test]
fn cg16_a_callee_precondition_is_the_callers_obligation() {
    if !clang_available() {
        return;
    }
    let decl = "int pick(int i); ";
    let c = contracts(PICK);
    assert_eq!(
        verdict(
            &format!("{decl}int subject(int j) {{ return pick(j); }}"),
            &c
        ),
        Unknown
    );
    assert_eq!(
        verdict(
            &format!("{decl}int subject(void) {{ return pick(2); }}"),
            &c
        ),
        Safe
    );
    assert_eq!(
        verdict(
            &format!("{decl}int subject(void) {{ return pick(7); }}"),
            &c
        ),
        Unsafe
    );
    assert_eq!(
        verdict(
            &format!(
                "{decl}int subject(int j) {{ if (j >= 0 && j < 4) return pick(j); return 0; }}"
            ),
            &c
        ),
        Safe
    );
    // Held: the obligation rests on a held declaration.
    assert_eq!(
        verdict(
            &format!("{decl}int subject(void) {{ return pick(2); }}"),
            &contracts(&held(PICK))
        ),
        Pending
    );
}

#[test]
fn cg16_byte_and_element_units_are_not_confused() {
    if !clang_available() {
        return;
    }
    let fill = "extern fn fill(p: ptr<mut, u8>) -> unit
contract {
    requires bounds_contract_v1;
    requires bounds_extent(p, 16, bytes);
};";
    let decl = "void fill(int *p); ";
    let c = contracts(fill);
    // `int` is 4 bytes on the recorded target: 4 ints are 16 bytes, 3 are 12.
    assert_eq!(
        verdict(
            &format!("{decl}void subject(void) {{ int a[4]; fill(a); }}"),
            &c
        ),
        Safe
    );
    assert_eq!(
        verdict(
            &format!("{decl}void subject(void) {{ int a[3]; fill(a); }}"),
            &c
        ),
        Unsafe
    );
    // A byte extent on a typed parameter, with a scaled index that overflows
    // the target's object-size limit: a named refusal, not a violation.
    let own = "extern fn subject(p: ptr<mut, u8>) -> i32
contract {
    requires bounds_contract_v1;
    requires bounds_extent(p, 16, bytes);
};";
    let c = contracts(own);
    assert_eq!(
        verdict("int subject(__INT64_TYPE__ *p) { return p[1]; }", &c),
        Safe
    );
    assert_eq!(
        verdict("int subject(__INT64_TYPE__ *p) { return p[2]; }", &c),
        Unsafe
    );
    let r = row(
        "int subject(__INT64_TYPE__ *p) { return p[1152921504606846976LL]; }",
        &c,
    );
    assert_eq!(r.verdict, "Unknown", "{r:#?}");
    assert!(has_reason(&r, "bounds-arithmetic-overflow"));
}

#[test]
fn cg16_a_return_extent_is_used_after_preconditions_and_withdrawn_on_release() {
    if !clang_available() {
        return;
    }
    let get = "extern fn get_buf(n: usize) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    ensures normal bounds_return_extent(n, elements);
};";
    let decl = "int *get_buf(unsigned long n); ";
    let c = contracts(get);
    assert_eq!(
        verdict(
            &format!("{decl}int subject(void) {{ int *p = get_buf(8); return p[7]; }}"),
            &c
        ),
        Safe
    );
    assert_eq!(
        verdict(
            &format!("{decl}int subject(void) {{ int *p = get_buf(8); return p[8]; }}"),
            &c
        ),
        Unknown,
        "a violation after a call is not definitely reached"
    );
    // A call that may release or resize what it reaches withdraws the fact.
    assert_eq!(
        verdict(
            &format!("{decl}int subject(void) {{ int *p = get_buf(8); q_free(p); return p[0]; }}"),
            &c
        ),
        Unknown
    );
    // A body that refutes its declared return extent: the declaration is
    // withdrawn from every caller in the unit.
    let fixed = "extern fn get_buf() -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    ensures normal bounds_return_extent(8, elements);
};";
    let c = contracts(fixed);
    let decl = "int *get_buf(void); ";
    assert_eq!(
        verdict(
            &format!("{decl}int subject(void) {{ int *p = get_buf(); return p[7]; }}"),
            &c
        ),
        Safe
    );
    let source = format!(
        "{decl}static int store[4]; int *get_buf(void) {{ return store; }} \
         int subject(void) {{ int *p = get_buf(); return p[0]; }}"
    );
    let unit_report = {
        let text = text(&source);
        analyze_bounds_c11(
            &C11TranslationUnit {
                source: &text,
                ..Default::default()
            },
            &["subject".to_owned(), "get_buf".to_owned()],
            &c,
        )
        .unwrap()
    };
    let getter = unit_report.row("get_buf").unwrap();
    assert!(getter.refutes_own_contract, "{getter:#?}");
    assert!(
        getter
            .reasons
            .iter()
            .any(|r| r.contains("bounds-contract-refuted-by-body"))
    );
    assert_eq!(unit_report.row("subject").unwrap().verdict, "Unknown");
}

// ---------------------------------------------------------------------------
// Composition and admission
// ---------------------------------------------------------------------------

#[test]
fn cg16_custody_and_bounds_rows_compose_only_when_both_are_requested() {
    if !clang_available() {
        return;
    }
    let both = Request {
        custody: true,
        bounds: true,
    };
    let cases = [
        ("int subject(int *p) { return *p; }", Unknown),
        ("int subject(void) { int arr[4]; return arr[7]; }", Unsafe),
        ("int subject(void) { int arr[4]; return arr[1]; }", Safe),
    ];
    for (source, combined) in cases {
        let custody = custody(source);
        assert_eq!(custody, Safe, "{source}");
        let bounds = verdict(source, &none());
        assert_eq!(
            compose(both, Some(custody), Some(bounds)),
            combined,
            "{source}"
        );
        // A custody-only request keeps its narrower verdict.
        assert_eq!(
            compose(
                Request {
                    custody: true,
                    bounds: false
                },
                Some(custody),
                None
            ),
            Safe
        );
        // A requested bounds row that is absent is Unknown, never omitted.
        assert_eq!(compose(both, Some(custody), None), Unknown);
    }
    let pending = row(RELATION_C, &contracts(&held(RELATION)));
    assert_eq!(
        compose(both, Some(Safe), Some(row_verdict(&pending))),
        Pending
    );
}

#[test]
fn cg16_admission_refuses_changed_or_incomplete_evidence() {
    if !clang_available() {
        return;
    }
    let source = "int subject(void) { int arr[4]; arr[1] = 0; return arr[2]; }";
    let full = text(source);
    let unit = C11TranslationUnit {
        source: &full,
        ..Default::default()
    };
    let selection = ["subject".to_owned()];
    let recorded = analyze_bounds_c11(&unit, &selection, &none()).unwrap();
    check_bounds_report(&recorded, &unit, &selection, &none()).expect("unchanged evidence admits");
    // A selected access missing from the evidence.
    let mut missing = recorded.clone();
    missing.functions[0].accesses.pop();
    assert!(check_bounds_report(&missing, &unit, &selection, &none()).is_err());
    // An omitted row.
    let mut omitted = recorded.clone();
    omitted.functions.clear();
    assert!(check_bounds_report(&omitted, &unit, &selection, &none()).is_err());
    // A changed contract environment.
    assert!(check_bounds_report(&recorded, &unit, &selection, &contracts(PICK)).is_err());
    // A changed target identity or malformed verdict.
    let mut target = recorded.clone();
    target.target = "another target".into();
    assert!(check_bounds_report(&target, &unit, &selection, &none()).is_err());
    let mut malformed = recorded.clone();
    malformed.functions[0].verdict = "Unsafe";
    assert!(check_bounds_report(&malformed, &unit, &selection, &none()).is_err());
    // A changed source.
    let other = text("int subject(void) { int arr[4]; arr[1] = 0; return arr[3]; }");
    let changed = C11TranslationUnit {
        source: &other,
        ..Default::default()
    };
    assert!(check_bounds_report(&recorded, &changed, &selection, &none()).is_err());
}

#[test]
fn cg16_unsupported_forms_are_named_unknowns() {
    if !clang_available() {
        return;
    }
    let r = row(
        "int subject(void) { int arr[4]; return ({ arr[1]; }); }",
        &none(),
    );
    assert_eq!(r.verdict, "Unknown");
    assert!(has_reason(&r, "bounds-unsupported-form"));
    let r = row("int subject(int n) { int v[n]; return v[0]; }", &none());
    assert_eq!(r.verdict, "Unknown");
    assert!(has_reason(&r, "CG17"), "{r:#?}");
}
