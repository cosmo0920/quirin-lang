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

//! Card CG00-CLANG-GAP-BASELINE, phase 3 (cards CG16–CG19): executed BEFORE
//! controls for the source side of the Clang gap analysis's connections to the
//! existing core checkers. Conventions as in `clang_gap_phase1.rs`.
//!
//! Most of these entries are **limits**, not refusals: the custody gateway
//! reads the access as a custody-neutral scalar and publishes `Safe` without
//! any extent, alignment, same-object, liveness-generation, effective-type or
//! lock obligation, because none is part of the custody guarantee. The test
//! pins exactly that, so an implementation card that adds the obligation
//! moves it deliberately. The core-model halves are in
//! `crates/qcore-core/tests/clang_gap_core.rs`.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct S { int a; int b; unsigned bf : 3; void *p; void *v; \
struct I { void *p; int x; } in; int arr[4]; void *slots[4]; void (*cb)(void); };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

// ---------------------------------------------------------------------------
// CG16 — index and offset extents (M14 M15 M16 M17)
// ---------------------------------------------------------------------------

#[test]
fn m14_computed_indices_carry_no_bounds_obligation() {
    if !clang_available() {
        return;
    }
    assert!(gs("int subject(struct S *s, int i) { return s->arr[i]; }").safe());
    // Out of the declared extent (`int arr[4]`), and still custody-Safe.
    assert!(gs("int subject(struct S *s) { return s->arr[7]; }").safe());
    // A loop over an array of resources is refused rather than bounded.
    let run = gs("void subject(void **a, int n) { int i; for (i = 0; i < n; i++) q_free(a[i]); }");
    assert!(run.refused(IND), "{run:?}");
}

#[test]
fn m15_pointer_arithmetic_carries_no_offset() {
    if !clang_available() {
        return;
    }
    assert!(gs("int subject(int *p, int i) { return *(p + i); }").safe());
    assert!(gs("void subject(void) { char *p = q_alloc(); char *q = p + 3; q_free(p); }").safe());
}

#[test]
fn m16_pointer_difference_and_ordering_carry_no_same_object_obligation() {
    if !clang_available() {
        return;
    }
    assert!(gs("long subject(char *a, char *b) { return a - b; }").safe());
    assert!(gs("int subject(char *a, char *b) { return a < b; }").safe());
}

#[test]
fn m17_one_past_formation_is_not_separated_from_dereference() {
    if !clang_available() {
        return;
    }
    // Formed and dereferenced one past `a + n`: custody-Safe.
    assert!(gs("int subject(int *a, int n) { int *e = a + n; return *e; }").safe());
    // Formed from an embedded array: refused (IND), not classified.
    assert!(
        gs("int subject(struct S *s) { int *e = s->arr + 4; return e == s->arr; }").refused(IND)
    );
}

// ---------------------------------------------------------------------------
// CG17 — layout-derived extents (M12 M13 M19 E18)
// ---------------------------------------------------------------------------

/// Correction to the input: the analysis cites a G "Unknown fixture for VLA
/// `sizeof` side effects". A VLA declaration itself is refused in G for a
/// different named reason: it is read as a declaration that may shadow a
/// formal resource identity.
#[test]
fn m12_variable_length_arrays_are_refused() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(int n) { int a[n]; return 0; }"),
        "local outside scalar/pointer callback profile"
    );
    // Flipped by CG17-LAYOUT-DERIVED-EXTENTS (rule 11u): a VLA of scalar
    // elements whose bound only reads and whose address never leaves its
    // block is no custody event; its numeric obligations are CG20's.
    for body in ["int a[n];", "int a[n]; a[0] = 1;"] {
        let run = gs(&format!("void subject(int n) {{ {body} }}"));
        assert!(run.safe(), "{body}: {run:?}");
    }
    // A VLA whose address may leave its block keeps the refusal.
    let escaping = gs("int *subject(int n) { int a[n]; return a; }");
    assert!(
        escaping.refused("local declaration may shadow a formal resource identity"),
        "{escaping:?}"
    );
    // Flipped by CG08-CASTS-AND-TYPE-QUERIES (rule 11b, E16): an evaluated
    // VLA type-query operand is read once (its size expression `n`).
    assert!(
        gs("unsigned long subject(int n) { unsigned long z = sizeof(int[n]); return z; }").safe()
    );
}

/// Still custody-Safe after CG17-LAYOUT-DERIVED-EXTENTS (rule 11u): the
/// flexible-array extent is a separate fact (`layout_report`, size not
/// established without CG16's vocabulary), never a custody obligation.
#[test]
fn m13_flexible_array_members_carry_no_extent() {
    if !clang_available() {
        return;
    }
    assert!(gs("struct F { int n; int data[]; }; int subject(struct F *f, int i) { return f->data[i]; }").safe());
}

/// Still custody-Safe after CG17-LAYOUT-DERIVED-EXTENTS (rule 11u): the
/// alignment obligation is the separate `layout.alignment.v1` property
/// (`layout_report`: undecided here, so that property is Unknown).
#[test]
fn m19_derived_addresses_carry_no_alignment_obligation() {
    if !clang_available() {
        return;
    }
    assert!(gs("int subject(char *c) { return *(int *)(c + 1); }").safe());
}

#[test]
fn e18_offsetof_and_container_of_are_refused() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        t_refusal(
            "struct R { int a; int b; }; unsigned long subject(void) { return __builtin_offsetof(struct R, b); }"
        ),
        "unmodeled callback expression OffsetOfExpr"
    );
    // Card CG21-CALL-EXITS-AND-BOUNDARIES (rule 11o): an extension node is
    // named, never an uncontracted call (BEFORE: "call to uncontracted
    // function `__builtin_offsetof`").
    assert!(gs("unsigned long subject(void) { unsigned long o = __builtin_offsetof(struct S, b); return o; }")
        .refused("`__builtin_offsetof` is the Clang extension node OffsetOfExpr, which no resource contract or route models"));
    assert!(gs("struct S *subject(int *m) { return (struct S *)((char *)m - __builtin_offsetof(struct S, b)); }")
        .refused(RETURN));
    // CG17-LAYOUT-DERIVED-EXTENTS (rule 11u): only a pointer established as
    // the member's address is the enclosing object; this one is unrelated
    // and stays refused (tests/layout_extents.rs has the admitted form).
}

// ---------------------------------------------------------------------------
// CG18 — footprint connection (M20 M21 M22 M26 W08)
// ---------------------------------------------------------------------------

#[test]
fn m20_stale_aliases_are_unresolved_not_generation_checked() {
    if !clang_available() {
        return;
    }
    // A copy used after the release: an unresolved identity, not a
    // use-after-release judgement.
    let run =
        gs("void subject(void) { char *p = q_alloc(); char *q = p; q_free(p); q_observe(q); }");
    assert!(
        run.unknown() && run.has("UnknownAlias { variable: \"q\" }"),
        "{run:?}"
    );
    // The old pointer after a successful reallocation.
    let run = gs(
        "void subject(void) { void *p = q_alloc(); void *n = q_realloc(p, 8); \
                  if (!n) { q_free(p); return; } q_observe(p); q_free(n); }",
    );
    assert!(
        run.unknown() && run.has("UnknownAlias { variable: \"p\" }"),
        "{run:?}"
    );
}

#[test]
fn m21_cross_allocation_accesses_need_no_disjointness_for_custody() {
    if !clang_available() {
        return;
    }
    assert!(gs("void subject(void) { char *p = q_alloc(); char *q = q_alloc(); *p = *q; q_free(p); q_free(q); }").safe());
    assert!(gs("void subject(void) { char *p = q_alloc(); char *q = q_alloc(); p[0] = q[0]; q_free(p); q_free(q); }").safe());
}

#[test]
fn m22_type_punning_and_restrict() {
    if !clang_available() {
        return;
    }
    // G reads a punned load as a neutral scalar: the effective-type premise is
    // implicit, not stated.
    assert!(gs("int subject(float *f) { return *(int *)f; }").safe());
    assert!(gs("int subject(struct S *s) { return *(int *)&s->p; }").refused(RETURN));
    assert_eq!(
        t_refusal("int subject(int *restrict p) { return *p; }"),
        "qualified access requires additional synchronization/alias evidence"
    );
}

#[test]
fn m26_memcpy_byte_semantics_are_a_contract_boundary() {
    if !clang_available() {
        return;
    }
    let run = gs("void *memcpy(void *d, const void *s, size_t n); \
                  void subject(void) { char *p = q_alloc(); memcpy(p, p + 1, 4); q_free(p); }");
    assert!(
        run.refused("call to uncontracted function `memcpy`"),
        "{run:?}"
    );
}

#[test]
fn w08_field_updates_carry_no_lock_invariant() {
    if !clang_available() {
        return;
    }
    assert!(
        gs("struct M { int lock; int count; }; void subject(struct M *m) { m->count++; }").safe()
    );
}

// ---------------------------------------------------------------------------
// CG19 — atomic source route (W01 W09)
// ---------------------------------------------------------------------------

/// Flipped by card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i). BEFORE: T
/// refused `_Atomic` under the volatile/restrict sentence and an atomic
/// builtin as an unmodeled `AtomicExpr`; G read an atomic builtin as an
/// uncontracted call. AFTER: T names the atomic arm and the route's answer; G
/// routes the plain load to the weak-memory vocabulary (custody-`Safe`, no
/// ordering claim).
#[test]
fn w01_atomic_accesses_do_not_reach_the_atomic_model() {
    if !clang_atomic_ast_available() {
        return;
    }
    assert_eq!(
        t_refusal("int subject(_Atomic int *p) { return *p; }"),
        "atomic-qualified type `int (_Atomic(int) *)` is read by the atomic source route, which no typed-source consumer decides (CG19)"
    );
    assert_eq!(
        t_refusal("int subject(int *p) { return __atomic_load_n(p, 5); }"),
        "atomic `__atomic_load_n` is routed to the weak-memory model, which no typed-source consumer decides (CG19)"
    );
    // G: an `_Atomic` load is a neutral scalar (unchanged); a plain atomic
    // builtin load is routed (CG19).
    assert!(gs("int subject(_Atomic int *a) { return *a; }").safe());
    assert!(gs("int subject(_Atomic int *a) { return __c11_atomic_load(a, 5); }").safe());
}

/// Flipped by card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i). BEFORE: G
/// refused the release store as an uncontracted call. AFTER: G routes it and
/// admits the function's custody, which says nothing about publication; the
/// publication judgment is the atomic-publication route's, which decides only
/// static flag/payload objects of one closed unit and refuses this
/// parameter-based shape (`crates/qcore-verification/tests/atomic_publication_source.rs`).
#[test]
fn w09_general_publication_is_not_derived_from_source() {
    if !clang_atomic_ast_available() {
        return;
    }
    let run = gs(
        "void subject(_Atomic int *flag, int *data) { *data = 1; __c11_atomic_store(flag, 1, 3); }",
    );
    assert!(run.safe(), "{run:?}");
}
