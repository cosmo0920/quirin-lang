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

//! Card CG00-CLANG-GAP-BASELINE, phase 2 (cards CG10–CG15): executed BEFORE
//! controls for the Clang gap analysis's typed-place and store entries.
//! Conventions as in `clang_gap_phase1.rs`; the core-model halves of M06 and
//! M07 are in `crates/qcore-core/tests/clang_gap_core.rs`.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct S { int a; int b; unsigned bf : 3; void *p; void *v; \
struct I { void *p; int x; } in; int arr[4]; void *slots[4]; void (*cb)(void); };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

// ---------------------------------------------------------------------------
// CG10 — common typed place (M01 I04; K03's place side is in phase 1)
// ---------------------------------------------------------------------------

#[test]
fn m01_mixed_paths_on_a_tracked_root_carry_per_hop_obligations() {
    if !clang_available() {
        return;
    }
    // CG10 flip (rule 10t): a tracked root (bound to an acquisition) reads
    // its embedded field and array element with the root's own obligation.
    // BEFORE: refused (IND), not Safe.
    let run = gs(
        "void subject(void) { struct S *s = q_alloc(); s->in.x = 1; s->arr[1] = 2; q_free(s); }",
    );
    assert!(run.safe(), "{run:?}");
    // Never-bound formal root: the same mixed path is read, as before.
    assert!(gs("int subject(struct S *s) { return s->in.x + s->arr[1]; }").safe());
    // Liveness comes from the custody state: after the release the place is
    // not read as live (an accusation or an unresolved alias, never Safe).
    let run = gs("void subject(void) { struct S *s = q_alloc(); q_free(s); s->in.x = 1; }");
    assert!(!run.safe(), "{run:?}");
    assert!(!run.refused(IND), "{run:?}");
    // A possibly-null root is not read until a test refines it.
    let run = gs(
        "void subject(void) { struct S *s = q_maybe_alloc(); s->in.x = 1; q_nullable_free(s); }",
    );
    assert!(!run.safe(), "{run:?}");
    let run = gs(
        "void subject(void) { struct S *s = q_maybe_alloc(); if (s) { s->in.x = 1; } q_nullable_free(s); }",
    );
    assert!(run.safe(), "{run:?}");
    // A pointer value read through the place is a borrowed alias with no
    // custody: reading it is admitted, releasing it is not the owner's release.
    let run = gs("void subject(void) { struct S *s = q_alloc(); void *q = s->in.p; q_free(s); }");
    assert!(run.safe(), "{run:?}");
    // A pointer-valued store stays refused (CG11).
    let run = gs("void subject(void *q) { struct S *s = q_alloc(); s->in.p = q; q_free(s); }");
    assert!(run.refused(IND), "{run:?}");
    // A root bound to a target-free value (an opaque read) is refused at the
    // flow, exactly as a direct `s->a` chain is.
    // Card NULL-SAFETY-PROPERTY (rule 14n, USER DECISION 2026-10-10,
    // decision 1) moves this row deliberately: `s` is a target-free,
    // possibly-null borrowed alias, so its dereference is read non-null under
    // the named premise on `s` the summary lists, and the scalar store
    // certifies resting on it (the null question is the null property's).
    let run = gs("void subject(struct S **v) { struct S *s = *v; s->in.x = 1; }");
    assert!(run.safe() && run.null_premises == ["s"], "{run:?}");
}

#[test]
fn i04_one_line_identity_is_the_gateway_occurrence() {
    if !clang_available() {
        return;
    }
    // CG10 flip (I04): two identical statements on one line are told apart
    // by the gateway's own occurrence of each. BEFORE: refused (IND).
    let run = gs("void subject(struct S *s) { s->in.x = 1; s->in.x = 1; }");
    assert!(run.safe(), "{run:?}");
    // The same statements on separate lines are read, as before.
    assert!(gs("void subject(struct S *s) {\n s->in.x = 1;\n s->in.x = 1;\n}").safe());
    // Identical condition operands on one line keep the unique-match
    // refusal: an operand is a copy with no stream occurrence (residual).
    let run = gs(
        "int subject(struct S *s, int t) { if (s->in.x == t) { cg_sink(1); } if (s->in.x == t) { cg_sink(2); } return 0; }",
    );
    assert!(run.refused(BRANCH), "{run:?}");
    // A single such operand is read.
    assert!(
        gs("int subject(struct S *s, int t) { if (s->in.x == t) { cg_sink(1); } return 0; }")
            .safe()
    );
}

// ---------------------------------------------------------------------------
// CG11 — pointer-valued stores (M02 M03 M04)
// ---------------------------------------------------------------------------

#[test]
fn m02_general_pointer_valued_stores_are_refused() {
    if !clang_available() {
        return;
    }
    // Example C's store `s->p = q`, a slot and an element.
    assert!(gs("void subject(struct S *s, void *q) { s->p = q; }").refused(IND));
    assert!(
        gs("void subject(void **slot, void *p) { *slot = p; }")
            .refused("store through `*slot` without an out-parameter contract")
    );
    assert!(
        gs("void subject(void **slots, int i, void *p) { slots[i] = p; }").refused(DESTINATION)
    );
    // Controls: the limited forms that exist — a local copy, a contracted
    // store, an Out hand-off of a fresh acquisition.
    assert!(gs("void subject(void) { void *a = q_alloc(); void *b = a; q_free(b); }").safe());
    assert!(gs("void subject(void) { void *owner = q_alloc(); void *item = q_alloc(); q_store(owner, item); q_free(owner); }").safe());
    assert!(gs("void subject(void **slot) { void *p = q_alloc(); *slot = p; }").safe());
}

#[test]
fn m03_field_stores_through_formals_and_local_owners_are_refused() {
    if !clang_available() {
        return;
    }
    let run = gs("void subject(struct S *s) { void *q = q_alloc(); s->p = q; }");
    assert!(run.refused(IND), "{run:?}");
    // A local owner acquired here is refused too in this shape.
    let run = gs(
        "void subject(void) { struct S *s = q_alloc(); void *q = q_alloc(); s->p = q; q_free(s->p); q_free(s); }",
    );
    assert!(run.refused(IND), "{run:?}");
}

#[test]
fn m04_nested_array_and_function_pointer_stores_are_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(void) { struct S *s = q_alloc(); void *q = q_alloc(); s->in.p = q; q_free(s); }",
        "void subject(void) { struct S *s = q_alloc(); void *q = q_alloc(); s->slots[1] = q; q_free(s); }",
    ] {
        let run = gs(body);
        assert!(run.refused(IND), "{body}: {run:?}");
    }
    // CG11 flip (rule 10v): a function stored into a caller's object moves no
    // custody. BEFORE: refused (IND).
    let run = gs("void f(void); void subject(struct S *s) { s->cb = f; }");
    assert!(run.safe(), "{run:?}");
}

// ---------------------------------------------------------------------------
// CG12 — typed readback from `void *` fields (M05)
// ---------------------------------------------------------------------------

/// CG12's correction to CG00's control: over this file's `struct S` (a
/// bit-field, arrays, a function pointer) the IND refusal is the *store*
/// `s->v = q_alloc()`, which ALIAS-AGGREGATE's record reader does not admit
/// for a record it did not read whole; the readback is never reached, so it
/// stays refused after CG12 too.
///
/// CG12 flip (rule 11j): over a record the reader reads, the same body is
/// `Safe`: the readback binds `c` to the identity the store linked, and
/// `q_free(c)` releases it. BEFORE: `UnknownAlias { variable: "c" }`.
#[test]
fn m05_readback_from_a_void_field_is_refused() {
    if !clang_available() {
        return;
    }
    let body = "void subject(void) { struct S *s = q_alloc(); s->v = q_alloc(); char *c = s->v; q_free(c); q_free(s); }";
    let run = gs(body);
    assert!(run.refused(IND), "{run:?}");
    assert!(run.has("UnknownAlias { variable: \"c\" }"));
    let run = g(&format!("struct S {{ int a; void *v; }};\n{body}"));
    assert!(run.safe(), "{run:?}");
}

// ---------------------------------------------------------------------------
// CG13 — unions and bit-fields (M06 M07)
// ---------------------------------------------------------------------------

/// CG00's partial correction to the input: G refused union and bit-field
/// *stores* and read a scalar union member and a bit-field through a
/// never-bound root as custody-neutral scalars.
///
/// CG13 flip (rule 10w): a scalar store through a union member is admitted,
/// with W43's refused-write frame because `union U` can hold a pointer, and a
/// union member in a member chain is a place step. BEFORE: the store was
/// refused (IND). A pointer-valued store through the union stays refused.
/// Route T is unchanged: no typed-source consumer decides a union member.
#[test]
fn m06_union_member_steps_are_read_by_rule_10w() {
    if !clang_available() {
        return;
    }
    const U: &str = "union U { int i; void *p; };";
    assert!(gs(&format!("{U} void subject(union U *u) {{ u->i = 1; }}")).safe());
    assert!(gs(&format!("{U} int subject(union U *u) {{ return u->i; }}")).safe());
    assert!(
        gs(&format!(
            "{U} struct W {{ union U u; }}; void subject(struct W *w) {{ w->u.i = 1; }}"
        ))
        .safe()
    );
    assert!(
        gs(&format!(
            "{U} void subject(union U *u) {{ u->p = q_alloc(); }}"
        ))
        .refused(IND)
    );
    assert_eq!(
        t_refusal("union U { int i; float f; }; int subject(union U *u) { return u->i; }"),
        "field is not a direct ordinary field of a selected record"
    );
}

/// CG13 flip (rule 10w): a bit-field store is a read-modify-write of its
/// storage unit, custody-neutral. BEFORE: refused (IND). Route T unchanged.
#[test]
fn m07_bitfield_steps_are_unit_accesses() {
    if !clang_available() {
        return;
    }
    assert!(gs("void subject(struct S *s) { s->bf = 1; }").safe());
    assert!(gs("int subject(struct S *s) { return s->bf; }").safe());
    // T drops bit-fields from its record field table.
    assert_eq!(
        t_with(
            "struct B { unsigned bf : 3; int a; }; int subject(struct B *b) { return b->bf; }",
            "subject",
            &["B"],
        )
        .expect_err("a bit-field is not a selected field"),
        "field is not a direct ordinary field of a selected record"
    );
}

// ---------------------------------------------------------------------------
// CG14 — aggregate values and initializers (M08 M09 M10 M11)
// ---------------------------------------------------------------------------

/// Unchanged by CG14 (rule 11f): `struct S` holds pointer members, so a whole
/// copy of it or of its embedded `struct I` refuses (`pointer-members`).
#[test]
fn m08_aggregate_copies_are_refused() {
    if !clang_available() {
        return;
    }
    assert!(gs("void subject(struct S *a, struct S *b) { *a = *b; }").refused(IND));
    assert!(gs("void subject(struct S *s) { s->in = s->in; }").refused(IND));
    // CG14 (rule 11f): a custody-free record's copy is its typed read and
    // store. Added control, not a flip.
    assert!(
        gs("struct R { int x; long y[2]; }; void subject(struct R *a, struct R *b) { *a = *b; }")
            .safe()
    );
}

#[test]
fn m09_braced_initializers_are_refused() {
    if !clang_available() {
        return;
    }
    // Unchanged: an initial alias (a local's pointer) refuses.
    let run =
        gs("void subject(void) { void *q = q_alloc(); struct S s = { .p = q }; q_free(s.p); }");
    assert!(run.refused(INITIALIZER), "{run:?}");
    // CG14 flip (rule 11f): every leaf carries no identity (a scalar, and the
    // zero-fill of the rest, pointers null). BEFORE: refused (INITIALIZER).
    assert!(gs("int subject(void) { struct S s = { .a = 1 }; return s.a; }").safe());
    // Route T is unchanged: no T consumer decides a wider initializer.
    assert_eq!(
        t_refusal(
            "struct R { int a; int b; }; int subject(void) { struct R r = { .a = 1 }; return 0; }"
        ),
        "local outside scalar/pointer callback profile"
    );
    // T's one initializer is the two-field `struct timeval`, and only with its
    // record selected.
    assert_eq!(
        t_refusal(
            "struct timeval { long tv_sec; long tv_usec; }; int subject(void) { struct timeval t = { 1, 2 }; return 0; }"
        ),
        "record initializer outside fixed timeval profile"
    );
}

#[test]
fn m10_compound_literals_have_no_identity() {
    if !clang_available() {
        return;
    }
    // CG14 flip (rule 11f): the literal's address at a `Borrowed` formal,
    // identity-free and reading nothing. BEFORE: refused (INITIALIZER).
    let run = gs(
        "void subject(void) { void *q = q_alloc(); q_observe(&(struct S){ .p = 0 }); q_free(q); }",
    );
    assert!(run.safe(), "{run:?}");
    // Route T is unchanged.
    assert_eq!(
        t_refusal(
            "struct R { int a; int b; }; int f(struct R *); int subject(void) { return f(&(struct R){1, 2}); }"
        ),
        "unmodeled callback expression CompoundLiteralExpr"
    );
}

/// Correction to the input (CG00): G did not read a string-initialised local
/// array at all (an unrecognized destination), and a pointer to a string
/// literal binds the literal as an unresolved identity.
#[test]
fn m11_string_initializers_are_refused_or_unresolved() {
    if !clang_available() {
        return;
    }
    // CG14 flips (rule 11f): a character array initialised by a string
    // literal is a declaration with no custody effect; its extent is
    // recorded, and no bound is claimed (`buf[9]` is read as any array
    // element is, custody-neutral). BEFORE: both refused (DESTINATION).
    assert!(gs("void subject(void) { char buf[] = \"abc\"; }").safe());
    assert!(gs("int subject(void) { char buf[] = \"abc\"; return buf[9]; }").safe());
    // Unchanged.
    let run = gs("int subject(void) { const char *s = \"abc\"; return s[3]; }");
    assert!(run.unknown() && run.has("__qcore_literal__"), "{run:?}");
}

// ---------------------------------------------------------------------------
// CG15 — storage duration and volatile (M24 M23)
// ---------------------------------------------------------------------------

#[test]
fn m24_static_storage_is_refused_or_unresolved() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int g_counter; int subject(void) { return g_counter; }"),
        "global or unresolved local reference"
    );
    // CG15 (rule 11h): T names the form it refuses. BEFORE: the generic
    // "unmodeled callback statement DeclStmt".
    assert_eq!(
        t_refusal("int subject(void) { static int s; return s; }"),
        "typed-source static- or thread-duration local is decided by no typed-source consumer (CG15)"
    );
    // CG15 flip (rule 11h): a braced static table is no event and reading it
    // yields a pointer with no identity. BEFORE: refused (INITIALIZER, RETURN).
    let run = gs(
        "const char *subject(int i) { static const char *names[] = {\"a\", \"b\"}; \
         return names[i & 1]; }",
    );
    assert!(run.safe(), "{run:?}");
    // CG15 flip (rule 11h): a thread-local array element's address is a
    // pointer with no identity. BEFORE: refused (IND, RETURN).
    let run = gs(
        "char *subject(void) { static _Thread_local char ret[2][8]; \
         cg_log(ret[1]); return ret[1]; }",
    );
    assert!(run.safe(), "{run:?}");
    let run = gs("void *g_slot; void subject(void) { g_slot = q_alloc(); }");
    assert!(run.refused(
        "publication of a tracked resource into the static-storage object `g_slot` is outside the resource projection"
    ), "{run:?}");
    let run = gs("void *g_slot; void subject(void) { q_free(g_slot); }");
    assert!(run.has("UnknownAlias { variable: \"g_slot\" }"), "{run:?}");
    // A thread-local scalar read is custody-neutral: no lifetime is modelled.
    assert!(gs("_Thread_local int t_count; int subject(void) { return t_count; }").safe());
}

#[test]
fn m23_volatile_accesses_are_not_counted_or_ordered() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(volatile int *p) { return *p; }"),
        "qualified access requires additional synchronization/alias evidence"
    );
    // G reads two volatile loads as one neutral scalar expression: no count or
    // order of observable accesses exists in the verdict.
    assert!(gs("int subject(volatile int *v) { return *v + *v; }").safe());
    assert!(gs("struct V { volatile int r; }; int subject(struct V *v) { return v->r; }").safe());
}
