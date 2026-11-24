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

//! Card K3-POINTER-VALUED-STORES (round 34, gateway rule 10v amended):
//! pointer arithmetic over the destination's own object, end to end through
//! the custody gateway (route G).
//!
//! Every `Safe` here is `Safe` over the synthetic `q_*` fixture contracts and
//! promotes nothing; every `Safe` certificate is rechecked by the harness.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct S { char *name; char *end; void *state; struct S *self; \
char *slots[4]; int n; };\n\
struct E { char *base; long len; };\nstruct V { struct E *io; int n; };\nint f(void);\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

fn gs_function(source: &str, function: &str) -> G {
    g_function(&format!("{S}{source}"), function)
}

// ---------------------------------------------------------------------------
// Admitted: arithmetic over a pointer the destination's object already holds
// ---------------------------------------------------------------------------

#[test]
fn arithmetic_over_the_destinations_own_value_is_admitted() {
    if !clang_available() {
        return;
    }
    for body in [
        // nanopb's `buf_read`: the place advanced by its own value.
        "void subject(struct S *s, int n) { s->name = s->name + n; }",
        "void subject(struct S *s, int n) { s->state = (char *) s->state + n; }",
        "void subject(struct S *s, int n) { s->name = n + s->name; }",
        "void subject(struct S *s) { s->name = s->name - 1; }",
        // snappy's `writer_set_expected_length`: another member of the object.
        "void subject(struct S *s, int n) { s->end = s->name + n; }",
        "void subject(struct S *s, int i) { s->slots[i] = s->slots[i] + 1; }",
        // Monkey's `mk_iov_consume`: the same element through a pointer member.
        "void subject(struct V *v, int i, int n) { v->io[i].base = (char *) v->io[i].base + n; }",
        // A loop advancing the same place.
        "void subject(struct S *s, int n) { while (n-- > 0) s->name = s->name + 1; }",
    ] {
        let run = gs(body);
        assert!(run.safe(), "{body}: {run:?}");
    }
}

// ---------------------------------------------------------------------------
// Refused: a new reference, a stale copy, a tracked or released owner
// ---------------------------------------------------------------------------

#[test]
fn arithmetic_that_may_add_a_reference_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    for body in [
        // Monkey's `request_set`: a formal's buffer (example C's missing
        // owner premise).
        "void subject(struct S *s, char *b, int n) { s->name = b + n; }",
        // Another object's pointer.
        "void subject(struct S *s, struct S *t, int n) { s->name = t->name + n; }",
        // A place read through a pointer past the root that is not the
        // destination.
        "void subject(struct S *s) { s->end = s->self->name + 1; }",
        "void subject(struct S *s) { s->self->name = s->name + 1; }",
        // A local copy (nanopb's `buf_write`): the copy may be stale.
        "void subject(struct S *s, int n) { char *d = s->name; s->name = d + n; }",
        // An integer round trip.
        "void subject(struct S *s, int n) { s->name = (char *) ((long) s->name + n); }",
        // A call in the index.
        "void subject(struct S *s) { s->slots[f()] = s->slots[f()] + 1; }",
    ] {
        let run = gs(body);
        assert!(run.unknown(), "{body}: {run:?}");
        assert!(run.refused(IND), "{body}: {run:?}");
    }
}

#[test]
fn a_tracked_owner_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(void) { struct S *o = q_alloc(); o->name = o->name + 1; q_free(o); }",
        "struct S *subject(void) { struct S *o = q_alloc(); o->end = o->name + 1; return o; }",
    ] {
        let run = gs(body);
        assert!(!run.safe(), "{body}: {run:?}");
        assert!(run.refused(IND), "{body}: {run:?}");
    }
}

#[test]
fn a_release_around_a_derived_store_is_never_safe() {
    if !clang_available() {
        return;
    }
    for body in [
        // The owner released first.
        "void subject(struct S *s) { q_free(s); s->name = s->name + 1; }",
        // A release through the advanced place.
        "void subject(struct S *s) { s->name = s->name + 1; q_free(s->name); }",
        // A release through an alias of the place taken before the store.
        "void subject(struct S *s) { char **pp = &s->name; s->name = s->name + 1; q_free(*pp); }",
        // A release of the read value's object, then the derived store.
        "void subject(struct S *s) { q_free(s->name); s->end = s->name + 1; }",
    ] {
        let run = gs(body);
        assert!(!run.safe(), "{body}: {run:?}");
    }
}

#[test]
fn a_body_with_a_derived_store_publishes_no_summary() {
    if !clang_available() {
        return;
    }
    let source = "void advance(struct S *s, int n) { s->name = s->name + n; }\n\
                  void subject(struct S *s) { advance(s, 1); }";
    let callee = gs_function(source, "advance");
    assert!(callee.safe(), "{callee:?}");
    let caller = gs_function(source, "subject");
    assert!(!caller.safe(), "{caller:?}");
}
