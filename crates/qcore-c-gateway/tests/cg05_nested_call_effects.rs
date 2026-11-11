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

//! Card CG05-NESTED-CALL-EFFECTS (gateway rule 10y): nested and sibling calls
//! with non-identity effects on route G, through card CG03's one application
//! point. Every admission is paired with its fail-closed controls; `Safe` is
//! over the synthetic trusted fixture contracts of card CG00.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

fn nested(callee: &str) -> String {
    format!("nested call `{callee}` inside a modeled call is outside the resource projection")
}

/// Example B: the inner release applies before the outer entry, and the
/// outer call receives the inner call's (scalar) value.
#[test]
fn an_inner_release_applies_before_the_outer_call() {
    if !clang_available() {
        return;
    }
    let run = g("void subject(void) { void *p = q_alloc(); cg_sink(r_release_code(p)); }");
    assert!(run.safe(), "{run:?}");
    // The CFG lists its nodes last first.
    assert_eq!(run.calls, ["cg_sink", "r_release_code", "q_alloc"]);
    // The inner effect is really applied: a second release is a double release.
    let run =
        g("void subject(void) { void *p = q_alloc(); cg_sink(r_release_code(p)); q_free(p); }");
    assert!(!run.safe(), "{run:?}");
    // Consumed likewise.
    let run = g("void subject(void) { void *p = q_alloc(); cg_sink(r_consume_code(p)); }");
    assert!(run.safe(), "{run:?}");
}

/// An owned inner result is bound to a reserved place and is the outer
/// actual, with its identity: a releasing outer call discharges it, a
/// borrowing one leaves it owned, and that is reported.
#[test]
fn an_owned_inner_result_is_the_outer_actual() {
    if !clang_available() {
        return;
    }
    let run = g("void subject(void) { void *p = q_alloc(); q_free(r_wrap(p)); q_free(p); }");
    assert!(run.safe(), "{run:?}");
    assert_eq!(run.calls, ["q_free", "q_free", "r_wrap", "q_alloc"]);
    // Borrowed only: the wrapper's fresh custody is never discharged.
    let run = g("void subject(void) { void *p = q_alloc(); q_observe(r_wrap(p)); q_free(p); }");
    assert!(!run.safe(), "{run:?}");
    assert!(!run.refused(&nested("r_wrap")), "{run:?}");
    assert!(run.has("Leak") || run.has("leak"), "{run:?}");
}

/// E11: nesting deeper than one level, and a call in part of an actual.
#[test]
fn deeper_and_partial_nested_calls_compose_inside_out() {
    if !clang_available() {
        return;
    }
    let run =
        g("void subject(void) { void *p = q_alloc(); q_free(r_wrap(r_wrap(p))); q_free(p); }");
    // The middle wrapper's fresh custody is only borrowed by the outer
    // wrapper: a leak, reported.
    assert!(!run.safe(), "{run:?}");
    assert!(!run.refused(&nested("r_wrap")), "{run:?}");
    let run =
        g("void subject(void) { void *p = q_alloc(); cg_sink(r_borrow_code(p) + 1); q_free(p); }");
    assert!(run.safe(), "{run:?}");
    let run = g(
        "void subject(void) { void *p = q_alloc(); void *w = r_wrap(p); \
         cg_sink(r_borrow_code(w) + 2 - r_borrow_code(p)); q_free(w); q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
}

/// E12: siblings commute by their footprints, or stay refused.
#[test]
fn siblings_are_admitted_only_when_their_footprints_commute() {
    if !clang_available() {
        return;
    }
    // Two fresh identities from borrowed reads commute: admitted, and the two
    // wrappers' custody, only borrowed, is reported.
    let run = g("void subject(void) { void *p = q_alloc(); \
         cg_sink2(r_borrow_code(r_wrap(p)), r_borrow_code(r_wrap(p))); q_free(p); }");
    assert!(!run.refused(&nested("r_borrow_code")), "{run:?}");
    assert!(!run.safe(), "{run:?}");
    // Two identity siblings over one name commute (card B04's older route).
    let run = g("void subject(void) { void *p = q_alloc(); \
         cg_sink2(r_borrow_code(p), r_borrow_code(p) + 1); q_free(p); }");
    assert!(run.safe(), "{run:?}");
    // `f(free_and_status(p), use(p))`: refused in both orders.
    let run = g(
        "void subject(void) { void *p = q_alloc(); cg_sink2(r_release_code(p), r_borrow_code(p)); }",
    );
    assert!(run.refused(&nested("r_release_code")), "{run:?}");
    let run = g(
        "void subject(void) { void *p = q_alloc(); cg_sink2(r_borrow_code(p), r_release_code(p)); }",
    );
    assert!(run.refused(&nested("r_borrow_code")), "{run:?}");
    // `h(p, g(p))` with `g` releasing `p`.
    let run = g("void subject(void) { void *p = q_alloc(); cg_pair(p, r_release_code(p)); }");
    assert!(run.refused(&nested("r_release_code")), "{run:?}");
    // Two releases of names that may alias.
    let run = g(
        "void subject(void) { void *p = q_alloc(); void *q = q_alloc(); \
         cg_sink2(r_release_code(p), r_release_code(q)); }",
    );
    assert!(run.refused(&nested("r_release_code")), "{run:?}");
    // A release and an owned result from a borrowed read of another name.
    let run = g(
        "void subject(void) { void *p = q_alloc(); void *q = q_alloc(); \
         cg_pair(r_wrap(q), r_release_code(p)); q_free(q); }",
    );
    assert!(run.refused(&nested("r_wrap")), "{run:?}");
}

/// What stays refused, with the older refusal byte for byte.
#[test]
fn unlisted_forms_keep_the_nested_refusal() {
    if !clang_available() {
        return;
    }
    // An uncontracted inner callee.
    let run = g(
        "int helper(void *p); void subject(void) { void *p = q_alloc(); cg_sink(helper(p)); q_free(p); }",
    );
    assert!(run.refused(&nested("helper")), "{run:?}");
    // A value holding calls at a custody formal.
    let run = g(
        "void subject(void) { void *p = q_alloc(); q_free((void *)(long)r_borrow_code(p)); q_free(p); }",
    );
    assert!(!run.safe(), "{run:?}");
    // A reallocated inner result.
    let run = g("void subject(void) { void *p = q_alloc(); q_free(q_realloc(p, 8)); }");
    assert!(run.refused(&nested("q_realloc")), "{run:?}");
    // A conditional inside the actual: FLIPPED by
    // CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z), which lowers it by
    // branching on `c` (the call applied on its own edge, `cg_sink(0)` on the
    // other); rule 10y still does not read it.
    let run = g(
        "void subject(int c) { void *p = q_alloc(); cg_sink(c ? r_borrow_code(p) : 0); q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
}

/// A body that spells the reserved place keeps every refusal.
#[test]
fn the_reserved_place_must_be_unspelled() {
    if !clang_available() {
        return;
    }
    let run = g(
        "void subject(void) { void *__qcore_nested_value_0 = 0; void *p = q_alloc(); \
         cg_sink(r_release_code(p)); (void)__qcore_nested_value_0; }",
    );
    assert!(run.refused(&nested("r_release_code")), "{run:?}");
}

/// E11/E12 in a scalar store: a scalar value holding calls, each applied
/// inside-out, then the store.
#[test]
fn a_scalar_value_holding_calls_is_stored_after_its_calls() {
    if !clang_available() {
        return;
    }
    let run = g(
        "int subject(void) { void *p = q_alloc(); int n = r_borrow_code(p) + r_borrow_code(p); \
         q_free(p); return n; }",
    );
    assert!(run.safe(), "{run:?}");
    let run =
        g("int subject(void) { void *p = q_alloc(); int n; n = r_release_code(p) + 1; return n; }");
    assert!(run.safe(), "{run:?}");
    // A release unordered with a borrow of the same name stays refused.
    let run = g(
        "int subject(void) { void *p = q_alloc(); int n = r_release_code(p) + r_borrow_code(p); \
         return n; }",
    );
    assert!(!run.safe(), "{run:?}");
}

/// The `return` position: a whole call with nested actuals is card
/// C-GATEWAY-RETURN-CALL-W4's desugaring; a scalar value holding calls, in a
/// scalar-result function, escapes nothing.
#[test]
fn a_return_composes_its_nested_calls() {
    if !clang_available() {
        return;
    }
    let run = g("int subject(void *q) { return r_release_code(r_wrap(q)); }");
    assert!(run.safe(), "{run:?}");
    let run = g("int subject(void *q) { return r_release_code(r_wrap(q)) + 1; }");
    assert!(run.safe(), "{run:?}");
    // The inner wrapper's custody is only borrowed by the outer one.
    let run = g("void *subject(void *q) { return r_wrap(r_wrap(q)); }");
    assert!(!run.safe(), "{run:?}");
    assert!(run.has("Leak"), "{run:?}");
    // Siblings touching one identity stay refused.
    let run = g(
        "int subject(void) { void *p = q_alloc(); return r_release_code(r_wrap(p)) + r_release_code(p); }",
    );
    assert!(!run.safe(), "{run:?}");
}

/// `CG04-LOOP-SITE-GENERATIONS`: one abstract site per acquisition node. A
/// nested owned result in a loop is either taken by the outer call in the same
/// iteration or left owned and reported; the reserved place is cleared, so
/// no later generation can stand in for an earlier one.
#[test]
fn a_nested_owned_result_in_a_loop_is_never_hidden() {
    if !clang_available() {
        return;
    }
    let run = g(
        "void subject(int n) { void *p = q_alloc(); for (int i = 0; i < n; i++) { \
         q_observe(r_wrap(p)); } q_free(p); }",
    );
    assert!(!run.safe(), "{run:?}");
    let run = g(
        "void subject(int n) { void *p = q_alloc(); for (int i = 0; i < n; i++) { \
         q_free(r_wrap(p)); } q_free(p); }",
    );
    assert!(!run.unsafe_(), "{run:?}");
}
