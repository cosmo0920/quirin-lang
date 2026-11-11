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

//! Card CG18-FOOTPRINT-CONNECTION (gateway rule 11k): the source route's
//! connection to the core footprint model, and its refusal controls, over the
//! CG00 harness (`support/clang_gap.rs`). A `Safe` here is custody `Safe` over
//! the synthetic fixture contracts and promotes nothing.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct I { void *p; int x; int y; }; \
struct S { int a; int b; unsigned bf : 3; unsigned bg : 2; char c; unsigned bh : 1; \
void *p; struct I in; int arr[4]; }; \
union U { int i; long l; }; struct W { union U u; int k; };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

/// Two members of one object, the same base on both sides: the core model
/// decides them disjoint from Clang's layout, and the pair is admitted.
#[test]
fn members_of_one_object_are_decided_by_the_core_model() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(struct S *s) { s->a = s->b++; return 0; }",
        "int subject(struct S *s) { int y; y = (s->a = 1) + (s->b = 2); return y; }",
        "int subject(struct S *s) { s->in.x = s->in.y++; return 0; }",
        "int subject(struct S *s) { s->in.x = s->a++; return 0; }",
        // A bit-field and an ordinary member, and bit-fields of two units.
        "int subject(struct S *s) { s->a = s->bf++; return 0; }",
        "int subject(struct S *s) { int y; y = (s->bf = 1) + (s->bh = 1); return y; }",
        "int subject(struct S *s) { int y; y = (s->bg = 1) + (s->c = 1); return y; }",
        // Through a typedef, whose pointer type Clang does not desugar: the
        // struct is read from the member's own declaration.
        "typedef struct P_s { long pid; signed char window; int hi; int lo; } P_t; \
         int subject(P_t *p) { p->lo = p->hi = p->window = 0; return 0; }",
    ] {
        let run = gs(source);
        assert!(run.safe(), "{source}: {run:?}");
    }
}

/// The refusal controls: every pair the core model does not decide keeps the
/// elaborator's refusal.
#[test]
fn every_other_pair_keeps_its_refusal() {
    if !clang_available() {
        return;
    }
    for source in [
        // Two allocations (two pointers): not assumed disjoint (M21, Q662).
        "int subject(struct S *s, struct S *t) { s->a = t->b++; return 0; }",
        "int subject(void) { struct S *s = q_alloc(); struct S *t = q_alloc(); \
         s->a = t->b++; q_free(s); q_free(t); return 0; }",
        // One member on both sides.
        "int subject(struct S *s) { s->in.x = s->in.x++; return 0; }",
        // A subscript index erased from the spelling: two elements.
        "int subject(struct S *s, int i, int j) { s->arr[i] = s->arr[j]++; return 0; }",
        // A subscript above the member: in bounds only under a premise.
        "int subject(struct S *s, int i) { s->arr[i] = s->a++; return 0; }",
        // A pointer hop above the member: another object.
        "int subject(struct S **v) { v[0]->a = v[1]->b++; return 0; }",
        // Two members of a union overlap.
        "int subject(struct W *w) { int y; y = (w->u.i = 1) + (w->u.l = 2); return y; }",
        // An unnamed struct: Clang's layout dump cannot be matched to it.
        "typedef struct { int a; int b; } A_t; int subject(A_t *p) { p->a = p->b++; return 0; }",
    ] {
        let run = gs(source);
        assert!(!run.safe(), "{source}: {run:?}");
    }
    // Two bit-fields of one storage unit (CG13's Q661 pair) stay refused.
    let run = gs("int subject(struct S *s) { int y; y = (s->bf = 1) + (s->bg = 2); return y; }");
    assert!(!run.safe(), "{run:?}");
}

/// The CG00 controls this card leaves as they are (M20, M21, M22, M26, W08):
/// custody verdicts rest on none of the footprint premises, so none moves.
#[test]
fn the_cg00_limits_are_unchanged() {
    if !clang_available() {
        return;
    }
    // M21: custody needs no disjointness across allocations.
    assert!(gs("void subject(void) { char *p = q_alloc(); char *q = q_alloc(); *p = *q; q_free(p); q_free(q); }").safe());
    // M22: a punned scalar load is custody-neutral; T refuses restrict.
    assert!(gs("int subject(float *f) { return *(int *)f; }").safe());
    assert_eq!(
        t_refusal("int subject(int *restrict p) { return *p; }"),
        "qualified access requires additional synchronization/alias evidence"
    );
    // W08: no lock invariant is claimed for a field update.
    assert!(
        gs("struct M { int lock; int count; }; void subject(struct M *m) { m->count++; }").safe()
    );
}
