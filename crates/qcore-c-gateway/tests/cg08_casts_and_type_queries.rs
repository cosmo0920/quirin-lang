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

//! Card CG08-CASTS-AND-TYPE-QUERIES (gateway rule 11b): casts as
//! representations and type-query operands by evaluation class on route G.
//! `Safe` here is over the synthetic trusted fixture contracts of the CG00
//! harness and promotes nothing.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct S { int a; int b; void *p; struct S *next; int arr[4]; };\n\
typedef unsigned long uintptr_t; typedef unsigned char BYTE; typedef _Bool bool;\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

const CAST_RETURN: &str =
    "cast of a possibly tracked value to another carrier in return expression";

// ---------------------------------------------------------------------------
// The soundness repair: a pointer converted to a truth value is no escape.
// ---------------------------------------------------------------------------

/// BEFORE (base `ca634dc1`): `return p;` in a `bool` function escaped `p`, and
/// `bool b = p;` bound `b` to `p`'s identity, so both leaks were Safe. The
/// explicit spellings were already Unsafe; the implicit ones now agree.
#[test]
fn a_pointer_returned_or_stored_as_a_truth_value_does_not_escape() {
    if !clang_available() {
        return;
    }
    for body in [
        "bool subject(void) { void *p = q_alloc(); return p; }",
        "bool subject(void) { void *p = q_alloc(); bool b = p; return b; }",
        "bool subject(void) { void *p = q_alloc(); bool b; b = p; return b; }",
        "bool subject(void) { void *p = q_alloc(); return (bool)p; }",
        "int subject(void) { void *p = q_alloc(); return p != 0; }",
    ] {
        let run = gs(body);
        assert!(run.unsafe_(), "{body}: {run:?}");
        assert!(run.has("Leak"), "{body}: {run:?}");
    }
    // The pointer is still released where the body releases it.
    for body in [
        "bool subject(void) { void *p = q_alloc(); bool b = p; q_free(p); return b; }",
        "bool subject(struct S *s) { return s->p; }",
        "bool subject(void *p) { return p; }",
    ] {
        let run = gs(body);
        assert!(!run.unsafe_(), "{body}: {run:?}");
        assert!(!run.has("CustodyConflict"), "{body}: {run:?}");
    }
}

/// A truth value stored into a member is data: the pointer is not published.
#[test]
fn a_truth_value_member_store_publishes_nothing() {
    if !clang_available() {
        return;
    }
    let run =
        gs("struct B { bool f; }; void subject(struct B *s) { void *p = q_alloc(); s->f = p; }");
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
    let run = gs(
        "struct B { bool f; }; void subject(struct B *s) { void *p = q_alloc(); s->f = p; q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
}

/// Residual `CG08-POINTER-TEST-CONDITION-REFINEMENT`: an explicit `(bool)p`
/// condition is admitted by an older route with no null refinement, so the
/// failure edge of a nullable acquisition is accused (the refusing-or-accusing
/// direction, never Safe). Pinned so that a card connecting it notices.
#[test]
fn an_explicit_truth_value_condition_is_not_yet_refined() {
    if !clang_available() {
        return;
    }
    let run =
        gs("void subject(void) { void *p = q_maybe_alloc(); if ((bool)p) q_nullable_free(p); }");
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
    let run = gs("void subject(void) { void *p = q_maybe_alloc(); if (p) q_nullable_free(p); }");
    assert!(run.safe(), "{run:?}");
}

// ---------------------------------------------------------------------------
// Pointer <-> integer (M18) and the cast return (CAST-RETURN-UNCOMMITTED-OPERAND)
// ---------------------------------------------------------------------------

/// Refusal control: a tracked pointer through `uintptr_t` and back, then
/// released, is refused; never Safe by identity.
#[test]
fn a_tracked_pointer_through_an_integer_is_refused_never_safe() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(void) { void *p = q_alloc(); uintptr_t v = (uintptr_t)p; void *q = (void *)v; q_free(q); }",
        "void subject(void) { void *p = q_alloc(); uintptr_t v; v = (uintptr_t)p; q_free((void *)v); }",
        "void subject(struct S *s) { void *p = q_alloc(); s->a = (int)(uintptr_t)p; q_free(p); }",
    ] {
        let run = gs(body);
        assert!(run.unknown(), "{body}: {run:?}");
    }
}

/// An untracked pointer's integer is data (a tag test, a stored address).
#[test]
fn an_untracked_pointer_integer_is_data() {
    if !clang_available() {
        return;
    }
    for body in [
        "unsigned long subject(void *p) { return (unsigned long)p & 7; }",
        "void subject(struct S *s, void *h) { s->a = (int)(uintptr_t)h; }",
        "unsigned long subject(void *h) { uintptr_t v = (uintptr_t)h; return v; }",
    ] {
        let run = gs(body);
        assert!(run.safe(), "{body}: {run:?}");
    }
}

/// `CAST-RETURN-UNCOMMITTED-OPERAND`: `void *` is a pointer the lexical
/// reader does not commit to; Clang's `PointerToIntegral` does.
#[test]
fn an_integer_cast_return_of_an_uncommitted_pointer_is_refused() {
    if !clang_available() {
        return;
    }
    let run = gs("unsigned long subject(void) { void *p = q_alloc(); return (unsigned long)p; }");
    assert!(run.unknown() && run.refused(CAST_RETURN), "{run:?}");
    // Untracked: the pre-card parse stands.
    assert!(gs("unsigned long subject(void *p) { return (unsigned long)p; }").safe());
}

// ---------------------------------------------------------------------------
// Pointer-to-pointer casts at a pointer destination (E15)
// ---------------------------------------------------------------------------

#[test]
fn an_explicit_object_pointer_cast_is_decided_as_the_uncast_statement() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(void *dst) { BYTE *d = (BYTE *)dst; d[0] = 1; }",
        "void subject(void *dst) { BYTE *d; d = (BYTE *)dst; d[0] = 1; }",
        "int subject(const void *a) { const struct S *s = (const struct S *)a; return s->a; }",
        "void subject(void) { void *p = q_alloc(); BYTE *c = (BYTE *)p; q_free(c); }",
    ] {
        let run = gs(body);
        assert!(run.safe(), "{body}: {run:?}");
    }
    // The alias keeps the identity: an unreleased one leaks.
    let run = gs("void subject(void) { void *p = q_alloc(); BYTE *c = (BYTE *)p; c[0] = 1; }");
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
    // Integer-to-pointer is no object-pointer conversion: refused.
    let run = gs("void subject(unsigned long v) { void *p = (void *)v; q_free(p); }");
    assert!(run.refused(ASSIGNMENT), "{run:?}");
}

// ---------------------------------------------------------------------------
// Type queries (E16, N11, example D)
// ---------------------------------------------------------------------------

#[test]
fn unevaluated_operands_read_nothing() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(struct S *s) { unsigned long n; n = sizeof(*s); cg_sink(n); }",
        "void subject(struct S *s) { unsigned long n = sizeof(s->next->a); cg_sink(n); }",
        "unsigned long subject(struct S *s) { return sizeof(s->a); }",
        "int subject(struct S *s, unsigned long n) { if (n < sizeof(*s)) return 1; return 0; }",
        "void subject(long v) { unsigned long z = (unsigned long)(sizeof(v)); cg_sink(z); }",
        "void subject(struct S *s) { void *p = q_alloc(); cg_sink(sizeof(s->arr)); q_free(p); }",
    ] {
        let run = gs(body);
        assert!(run.safe(), "{body}: {run:?}");
    }
    // A released pointer inside an unevaluated operand is not a use.
    let run = gs(
        "unsigned long subject(void) { void *p = q_alloc(); q_free(p); return sizeof(*(char *)p); }",
    );
    assert!(run.safe(), "{run:?}");
}

/// Refusal control: an evaluated VLA operand with an effectful call.
#[test]
fn an_evaluated_operand_with_an_effect_is_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "int f(void); unsigned long subject(void) { return sizeof(int[f()]); }",
        "int f(void); unsigned long subject(void) { unsigned long z = sizeof(int[f()]); return z; }",
        "unsigned long subject(int n) { unsigned long z = sizeof(char[n++]); return z; }",
        "void subject(int n) { void *p = q_alloc(); unsigned long z = sizeof(int[(q_free(p), n)]); cg_sink(z); }",
    ] {
        let run = gs(body);
        assert!(!run.safe(), "{body}: {run:?}");
        assert!(!run.unsupported.is_empty(), "{body}: {run:?}");
    }
    // The same VLA operand with a read only is evaluated once.
    assert!(
        gs("unsigned long subject(int n) { unsigned long z = sizeof(int[n]); return z; }").safe()
    );
}

/// Refusal control (example D): the marker `1` never reaches a bound or a
/// branch. No route decides a branch or a loop bound from a query's number,
/// so both edges of `sizeof(char) == 1` stay reachable and a loop bounded by
/// `sizeof(char)` is not one iteration.
#[test]
fn the_marker_never_reaches_a_bound_or_a_branch() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(void) { void *p = q_alloc(); if (sizeof(char) == 1) q_free(p); }",
        "void subject(void) { void *p = q_alloc(); if (sizeof(int) == 1) { } else { q_free(p); } }",
        "void subject(void) { void *p = q_alloc(); unsigned long i; for (i = 0; i < sizeof(char); i++) { q_free(p); } }",
        "void subject(void) { void *p = q_alloc(); unsigned long n = sizeof(char); while (n) { q_free(p); n = 0; } }",
    ] {
        let run = gs(body);
        assert!(!run.safe(), "{body}: {run:?}");
    }
}

/// `_Alignof` is never evaluated; a scalar type's is admitted.
#[test]
fn alignof_is_a_type_query() {
    if !clang_available() {
        return;
    }
    assert!(gs("unsigned long subject(struct S *s) { unsigned long a = _Alignof(struct S); return a + sizeof(*s); }").safe());
}

// ---------------------------------------------------------------------------
// Scalar representation rows (E15, N08) and the nanopb comma return
// ---------------------------------------------------------------------------

#[test]
fn scalar_representations_are_values_with_no_custody() {
    if !clang_available() {
        return;
    }
    for body in [
        "bool subject(int a) { bool b; b = a & 4; return b; }",
        "void subject(int n) { double d; d = n; cg_sink((int)d); }",
        "void subject(double d) { float f; f = d; cg_sink(f > 0); }",
        "void subject(double d) { bool b; b = d; cg_sink(b); }",
        "void subject(int n) { (void)(n + 1); }",
    ] {
        let run = gs(body);
        assert!(run.safe(), "{body}: {run:?}");
    }
}

/// `CG02-COMMA-OPERAND-RESIDUALS` (nanopb's `PB_RETURN_ERROR`): past the
/// `IntegralToBoolean` this card reads, the return still refuses (rule 10z's
/// position, then CG11's pointer member store).
#[test]
fn the_nanopb_comma_return_still_refuses_past_the_boolean_conversion() {
    if !clang_available() {
        return;
    }
    let run = gs("struct stream { const char *errmsg; }; \
         bool subject(struct stream *stream) { \
         return (stream->errmsg = (stream)->errmsg ? (stream)->errmsg : (\"x\")), 0; }");
    assert!(run.unknown(), "{run:?}");
}

/// N11 on route T: an implicit enumerator's value is Clang's evaluation
/// (BEFORE: refused as "enum constant requires an explicit evaluated
/// initializer"); an explicit one keeps its `ConstantExpr`.
#[test]
fn an_implicit_enumerator_is_clangs_constant_on_route_t() {
    if !clang_available() {
        return;
    }
    let summary =
        t("enum E { E0, E1 = 5, E2 }; int subject(void) { return E2; }").expect("admitted");
    let rendered = format!("{:?}", summary.paths());
    assert!(rendered.contains("Integer { value: \"6\" }"), "{rendered}");
    let summary =
        t("enum E { E0, E1 = 5, E2 }; int subject(void) { return E1; }").expect("admitted");
    let rendered = format!("{:?}", summary.paths());
    assert!(rendered.contains("Integer { value: \"5\" }"), "{rendered}");
}
