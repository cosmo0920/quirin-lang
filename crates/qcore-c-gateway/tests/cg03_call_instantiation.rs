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

//! Card CG03-CALL-SUMMARY-INSTANTIATION (gateway rule 10u): the single
//! call-contract application point, over route G with the CG00 fixture
//! contracts (synthetic trusted test assumptions, not library facts).
//!
//! Each admission is paired with the fail-closed controls that keep its
//! premises: a position that uses a call's value the same way accepts the
//! same calls, and nothing else moves.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct S { int a; int b; unsigned bf : 3; void *p; void *v; \
struct I { void *p; int x; } in; int arr[4]; void *slots[4]; void (*cb)(void); };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

/// A local callee outside the caller's recursive component, whose summary
/// is validated before the caller is analysed.
const LOCAL: &str = "static int loc(void *p) { return p != 0; }\n";

// ---------------------------------------------------------------------------
// K12: one acceptance range in the condition leaf and the return operand
// ---------------------------------------------------------------------------

#[test]
fn k12_a_local_callee_is_accepted_in_a_return_operand_as_in_a_condition() {
    if !clang_available() {
        return;
    }
    // The condition leaf (rule 5h) already took it.
    let run = gs(&format!(
        "{LOCAL}int subject(void *p) {{ int r = 0; if (loc(p) != 0) {{ r = 1; }} return r; }}"
    ));
    assert!(run.safe(), "{run:?}");
    // The return operand now takes the same call (BEFORE: the return
    // refusal; B01 took only an externally declared callee).
    for body in [
        "return loc(p) != 0;",
        "return !loc(p);",
        "return (loc(p) & 7) % 3;",
        // Two calls, each the identity (rule 5h's premise).
        "return loc(p) + loc(p);",
        "return r_borrow_code(p) + r_borrow_code(p);",
    ] {
        let run = gs(&format!("{LOCAL}int subject(void *p) {{ {body} }}"));
        assert!(run.safe(), "{body}: {run:?}");
        assert!(!run.refused(RETURN), "{body}: {run:?}");
    }
}

/// The statement position's typed argument reads (W44) are part of the one
/// application: a call whose actual is a typed sub-object address or a
/// member read is taken in a condition leaf and a return operand exactly as
/// in a statement (BEFORE: both refused as the gate's indirect access).
#[test]
fn k12_argument_reads_are_part_of_the_one_application() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(struct S *s) { r_borrow_code(&s->a); }",
        "int subject(struct S *s) { int r = 0; if (r_borrow_code(&s->a) > 0) { r = 1; } return r; }",
        "int subject(struct S *s) { return r_borrow_code(&s->a) > 0; }",
        "int subject(struct S *s) { return cg_sink2(s->a, 1), 0; }",
    ] {
        let run = gs(body);
        assert!(run.safe(), "{body}: {run:?}");
    }
    // The reads keep their obligation: a released base is accused.
    let run = gs(
        "int subject(void) { struct S *s = q_alloc(); q_free(s); return r_borrow_code(&s->a) > 0; }",
    );
    assert!(!run.safe(), "{run:?}");
    // A custody-taking formal still refuses a place (by design).
    let run = gs("int subject(struct S *s) { return r_release_code(s->p) > 0; }");
    assert!(!run.safe(), "{run:?}");
}

#[test]
fn k12_the_return_operand_applies_the_call_once_and_escapes_nothing() {
    if !clang_available() {
        return;
    }
    // The call's effect is applied: releasing an already released object is
    // accused through the new route.
    let run =
        gs("int subject(void) { void *p = q_alloc(); q_free(p); return r_release_code(p) == 0; }");
    assert!(run.unsafe_(), "{run:?}");
    // A released formal is its summary's `Released`, and the function is Safe.
    let run = gs("int subject(void *p) { return r_release_code(p) == 0; }");
    assert!(run.safe(), "{run:?}");
    // Nothing escapes through a scalar value: a held object leaks.
    let run = gs("int subject(void) { void *p = q_alloc(); return r_borrow_code(p) + 1; }");
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
}

#[test]
fn k12_the_return_operand_keeps_every_refusal_of_the_condition_leaf() {
    if !clang_available() {
        return;
    }
    for (body, why) in [
        // An owned result tested owns something (CG04 owns acquisition in a
        // tested position).
        (
            "int subject(void *p) { return r_wrap(p) != 0; }",
            "custody result",
        ),
        // Two calls of one operand, one of them not the identity.
        (
            "int subject(void *p, void *q) { return r_release_code(p) + r_borrow_code(q); }",
            "unsequenced effects",
        ),
        // A callee in the caller's recursive component may carry a
        // provisional summary.
        (
            "static int back(void *p); int subject(void *p) { return back(p) + 1; } \
             static int back(void *p) { return subject(p); }",
            "recursive component",
        ),
        // An uncontracted callee.
        (
            "int unknown_callee(void *p); int subject(void *p) { return unknown_callee(p) + 1; }",
            "uncontracted",
        ),
    ] {
        let run = gs(body);
        assert!(!run.safe(), "{why}: {run:?}");
    }
    // FLIPPED by CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z): a
    // connective return, CG03's "value connective" refusal row, is lowered by
    // branching on the connective (the condition CFG applies the call once on
    // the edge C evaluates it), each edge returning `1` or `0`.
    let run = gs("int subject(void *p, int n) { return r_borrow_code(p) && n; }");
    assert!(run.safe(), "{run:?}");
    // A pointer-result function's return is not a scalar operand.
    let run = gs("char *subject(char *s) { return s + r_borrow_code(s); }");
    assert!(!run.safe(), "{run:?}");
    // Rule 5j's return premise: the returned value takes no address.
    let run =
        gs("long subject(void *p) { long local = 0; return r_borrow_code(p) + (long) &local; }");
    assert!(run.refused(RETURN), "{run:?}");
    // The reserved return place spelled by the body keeps the refusal.
    let run = gs(
        "int subject(void) { void *__qcore_returned_value = q_alloc(); \
         return r_release_code(__qcore_returned_value) + 1; }",
    );
    assert!(run.refused(RETURN), "{run:?}");
}

// ---------------------------------------------------------------------------
// The elaborator's call whose actuals carry effects (Refusal::EffectfulActual)
// ---------------------------------------------------------------------------

#[test]
fn an_effectful_scalar_actual_is_sequenced_before_the_call() {
    if !clang_available() {
        return;
    }
    // BEFORE: the elaborator refused `cg_sink(i++)` by name (CG03).
    let run = gs("void subject(void) { int i = 0; cg_sink(i++); cg_sink(i); }");
    assert!(run.safe(), "{run:?}");
    let run = gs("void subject(void) { int i = 0; int j = 0; cg_sink2(i++, j += 2); cg_sink(j); }");
    assert!(run.safe(), "{run:?}");
    // A scalar path actual beside it: its read keeps its obligation.
    let run = gs("void subject(struct S *s) { int i = 0; cg_sink2(i++, s->a); }");
    assert!(run.safe(), "{run:?}");
    let run = gs(
        "void subject(void) { struct S *s = q_alloc(); int i = 0; q_free(s); cg_sink2(i++, s->a); }",
    );
    assert!(!run.safe(), "{run:?}");
    // The value stored into a scalar local.
    let run = gs("static int twice(int v) { return v * 2; } \
         void subject(void) { int i = 0; int r = 0; r = twice(i++); cg_sink(r); }");
    assert!(run.safe(), "{run:?}");
    // An unsequenced write/read pair on one object is C undefined behaviour.
    let run = gs("void subject(void) { int i = 0; cg_sink2(i++, i); }");
    assert!(!run.safe(), "{run:?}");
    // A pointer increment is pointer arithmetic (CG16).
    let run = gs("void subject(void) { char *p = q_alloc(); q_observe(p++); q_free(p); }");
    assert!(!run.safe(), "{run:?}");
    // A nested call stays card CG05's.
    let run = gs("void subject(void) { int i = 0; cg_sink(i++ + opaque()); }");
    assert!(!run.safe(), "{run:?}");
}

// ---------------------------------------------------------------------------
// K03: typed formal substitution
// ---------------------------------------------------------------------------

#[test]
fn k03_a_value_preserving_pointer_cast_substitutes_its_object() {
    if !clang_available() {
        return;
    }
    // BEFORE: `arguments do not match resource contract for `q_free``.
    let run = gs("void subject(void *ptr) { q_free((char *)ptr); }");
    assert!(run.safe(), "{run:?}");
    // The substituted identity is the object's: a second release is accused.
    let run = gs("void subject(void) { void *p = q_alloc(); q_free((struct S *)p); q_free(p); }");
    assert!(run.unsafe_(), "{run:?}");
    // Pointer arithmetic is not an identity (CG16): the refusal stays.
    let run = gs("void subject(void) { char *p = q_alloc(); q_free((char *)p + 1); }");
    assert!(!run.safe(), "{run:?}");
}

#[test]
fn k03_complex_places_at_custody_formals_stay_refused_by_design() {
    if !clang_available() {
        return;
    }
    // `W44-CALL-ARGUMENT-CUSTODY-POSITION`, and CG11's pointer store.
    for body in [
        "void subject(struct S *s) { q_free(s->in.p); }",
        "void subject(struct S *s, int i) { q_free(s->slots[i]); }",
        "void subject(struct S *s) { q_make(&s->p); }",
    ] {
        let run = gs(body);
        assert!(run.refused(IND), "{body}: {run:?}");
    }
    assert!(gs("void subject(void) { void *p; q_make(&p); q_free(p); }").safe());
}
