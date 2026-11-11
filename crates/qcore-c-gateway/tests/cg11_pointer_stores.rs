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

//! Card CG11-POINTER-VALUED-STORES (gateway rule 10v): pointer-valued stores
//! through the custody gateway (route G), end to end.
//!
//! Every `Safe` here is `Safe` over the synthetic `q_*` fixture contracts and
//! promotes nothing. The decision vectors of `src/pointer_store.rs` (shared
//! with `proof/QCore/PointerStore.lean`) are bound to the route's outcome by
//! [`every_decision_vector_is_the_route_outcome`].

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

/// A record whose bit-field and function-pointer members make W45's lexical
/// record reader refuse every member (W45-SUB-OBJECT-STORE-MEMBER-UNRESOLVED).
const S: &str = "struct I { void *p; int x; };\n\
struct S { int a; unsigned bf : 3; void *p; char *name; struct I in; void (*cb)(void); \
struct S *self; void **pp; void *slots[4]; };\nvoid f(void); void *cg_get(void); void (*g)(void);\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

fn gs_function(source: &str, function: &str) -> G {
    g_function(&format!("{S}{source}"), function)
}

// ---------------------------------------------------------------------------
// Admitted: a value with no identity into an untracked owner
// ---------------------------------------------------------------------------

#[test]
fn a_null_string_or_function_into_a_callers_object_is_admitted() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(struct S *s) { s->p = 0; }",
        "void subject(struct S *s) { s->p = (void *) 0; }",
        "void subject(struct S *s) { s->in.p = 0; }",
        "void subject(struct S *s) { s->name = \"x\"; }",
        "void subject(struct S *s) { s->cb = f; }",
        "void subject(struct S *s) { s->cb = &f; }",
        "void subject(struct S *s) { s->cb = g; }",
        "void subject(struct S *s, int i) { s->slots[i] = 0; }",
    ] {
        let run = gs(body);
        assert!(run.safe(), "{body}: {run:?}");
    }
}

#[test]
fn a_tailq_init_expansion_on_a_formals_member_is_admitted() {
    if !clang_available() {
        return;
    }
    // `TAILQ_INIT(&q->head)` as librdkafka's queue.h expands it.
    let run = gs(
        "struct E; struct H { struct E *tqh_first; struct E **tqh_last; };\n\
         struct Q { int n; unsigned bf : 1; struct H head; };\n\
         void subject(struct Q *q) { do { ((&q->head)->tqh_first) = ((void *) 0); \
         (&q->head)->tqh_last = &((&q->head)->tqh_first); } while (0); q->n = 0; }",
    );
    assert!(run.safe(), "{run:?}");
}

#[test]
fn an_untagged_typedef_root_is_read_by_its_declaration() {
    if !clang_available() {
        return;
    }
    // W45-SUB-OBJECT-STORE-ROOT-UNRESOLVED: W45's reader resolves no tag for
    // an untagged typedef; rule 10v reads the root by its Clang declaration.
    let run =
        gs("typedef struct { void *p; int x; } U; void subject(U *u) { u->p = 0; u->x = 1; }");
    assert!(run.safe(), "{run:?}");
}

#[test]
fn a_list_head_pointing_at_itself_is_admitted() {
    if !clang_available() {
        return;
    }
    // `mk_list_init`: the stored value is the destination's own object.
    let run = gs("struct L { struct L *prev, *next; };\n\
         void subject(struct L *list) { list->next = list; list->prev = list; }");
    assert!(run.safe(), "{run:?}");
}

// ---------------------------------------------------------------------------
// CG00's BEFORE controls: the M04 flip, and what stays refused
// ---------------------------------------------------------------------------

#[test]
fn m04_a_function_pointer_store_flips_and_the_owned_stores_stay_refused() {
    if !clang_available() {
        return;
    }
    // CG11 flip (rule 10v): `s->cb = f`, a function into a caller's object.
    // BEFORE: refused (IND).
    assert!(gs("void subject(struct S *s) { s->cb = f; }").safe());
    // A tracked identity into a local owner's nested member or element stays
    // refused (the owner is tracked; the value is a local's pointer).
    for body in [
        "void subject(void) { struct S *s = q_alloc(); void *q = q_alloc(); s->in.p = q; q_free(s); }",
        "void subject(void) { struct S *s = q_alloc(); void *q = q_alloc(); s->slots[1] = q; q_free(s); }",
    ] {
        let run = gs(body);
        assert!(run.refused(IND), "{body}: {run:?}");
    }
}

// ---------------------------------------------------------------------------
// Refusal controls the brief names
// ---------------------------------------------------------------------------

#[test]
fn example_c_a_store_into_a_formals_object_without_an_owner_premise_is_refused() {
    if !clang_available() {
        return;
    }
    // Q36's example C. No contract or tracked state gives the owner of `*s`
    // or bounds `*q`'s lifetime by it: the missing premise.
    let run = gs("void subject(struct S *s, void *q) { s->p = q; }");
    assert!(run.unknown() && run.refused(IND), "{run:?}");
    // An owned identity stored into the caller's object (OWNED-VALUE).
    let run = gs("void subject(struct S *s) { void *q = q_alloc(); s->p = q; }");
    assert!(run.unknown() && run.refused(IND), "{run:?}");
    // A local pointer that carries no identity is refused all the same.
    let run = gs("void subject(struct S *s) { void *l = 0; s->p = l; }");
    assert!(run.unknown(), "{run:?}");
}

#[test]
fn an_overwrite_of_an_owning_field_is_refused() {
    if !clang_available() {
        return;
    }
    // The owner is one tracked identity: its member may own the old value.
    for body in [
        "void subject(void) { struct S *s = q_alloc(); s->p = 0; q_free(s); }",
        "void subject(void) { struct S *s = q_alloc(); s->in.p = 0; q_free(s); }",
        "void subject(void) { struct S *s = q_alloc(); s->self = s; q_free(s); }",
    ] {
        let run = gs(body);
        assert!(run.unknown() && run.refused(IND), "{body}: {run:?}");
    }
}

#[test]
fn a_double_release_through_a_stored_alias_is_never_safe() {
    if !clang_available() {
        return;
    }
    // The stored alias of a tracked identity: refused, never Safe.
    let run =
        gs("void subject(struct S *n) { void *b = q_alloc(); n->p = b; q_free(n->p); q_free(b); }");
    assert!(!run.safe(), "{run:?}");
    // STABILITY-REEVAL-5's MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE shape with
    // the member stored first: the store refuses, the function stays Unknown.
    let run = gs(
        "void node_destroy(struct S *n) { if (n->p) q_nullable_free(n->p); q_nullable_free(n); }\n\
         void subject(void) { struct S *n = q_alloc(); void *b = q_alloc(); n->p = b; \
         q_nullable_free(n->p); node_destroy(n); }",
    );
    assert!(!run.safe(), "{run:?}");
}

#[test]
fn a_released_or_bound_owner_is_refused() {
    if !clang_available() {
        return;
    }
    let run = gs("void subject(struct S *d) { q_free(d); d->p = 0; }");
    assert!(!run.safe(), "{run:?}");
    let run = gs("void subject(struct S **v) { struct S *s = *v; s->p = 0; }");
    assert!(!run.safe(), "{run:?}");
}

#[test]
fn a_body_with_a_pointer_store_publishes_no_summary() {
    if !clang_available() {
        return;
    }
    let source = "void init(struct S *s) { s->p = 0; s->a = 0; }\n\
                  void subject(void) { struct S *s = q_alloc(); init(s); q_free(s); }";
    let callee = gs_function(source, "init");
    assert!(callee.safe(), "{callee:?}");
    let caller = gs_function(source, "subject");
    assert!(!caller.safe(), "{caller:?}");
    assert!(
        caller.refused("call to uncontracted function `init`"),
        "{caller:?}"
    );
    // The same caller of a scalar-only initialiser is served by its summary:
    // the gate withholds only what a pointer store would have claimed.
    let source = "void init(struct S *s) { s->a = 0; }\n\
                  void subject(void) { struct S *s = q_alloc(); init(s); q_free(s); }";
    assert!(gs_function(source, "subject").safe());
}

#[test]
fn values_that_are_reads_calls_arithmetic_or_other_addresses_are_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(struct S *s, struct S *t) { s->p = t->p; }",
        "void subject(struct S *s, struct S *t) { s->pp = &t->p; }",
        "void subject(struct S *s) { s->name = s->name + 1; }",
        "void subject(struct S *s) { s->p = cg_get(); }",
        "void subject(struct S *s) { int x = 0; s->p = &x; }",
        "void subject(struct S *s) { s->p = s->self ? (void *) 0 : (void *) 0; }",
    ] {
        let run = gs(body);
        assert!(!run.safe(), "{body}: {run:?}");
    }
}

// ---------------------------------------------------------------------------
// The shared decision vectors, bound to the route
// ---------------------------------------------------------------------------

/// The rows of `DECISION_VECTORS` in `src/pointer_store.rs`, read from its
/// text: `(spelling, value, carries, owner, decision)`.
fn vectors() -> Vec<(String, String, String, String)> {
    let text = include_str!("../src/pointer_store.rs");
    let table = text
        .split("pub(crate) const DECISION_VECTORS: &[(&str, &str, bool, &str, &str)] = &[")
        .nth(1)
        .expect("DECISION_VECTORS");
    let table = &table[..table.find("\n];").expect("end of vectors")];
    // Every field is a Rust string literal or `false`/`true`; read them in
    // order, five to a row.
    let mut fields = Vec::new();
    let mut chars = table.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                let mut literal = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => literal.push(chars.next().expect("escape")),
                        '"' => break,
                        other => literal.push(other),
                    }
                }
                fields.push(literal);
            }
            't' | 'f' if fields.len() % 5 == 2 => {
                let mut word = c.to_string();
                while chars.peek().is_some_and(char::is_ascii_alphabetic) {
                    word.push(chars.next().expect("letter"));
                }
                fields.push(word);
            }
            _ => {}
        }
    }
    assert_eq!(fields.len() % 5, 0, "{fields:?}");
    fields
        .chunks(5)
        .map(|row| {
            (
                row[0].clone(),
                row[1].clone(),
                row[3].clone(),
                row[4].clone(),
            )
        })
        .collect()
}

#[test]
fn every_decision_vector_is_the_route_outcome() {
    if !clang_available() {
        return;
    }
    let rows = vectors();
    assert_eq!(rows.len(), 17);
    for (spelling, value, owner, decision) in rows {
        let body = match owner.as_str() {
            "untracked" => format!(
                "void subject(struct S *s, void *q, struct S *t) {{ void *l = 0; {spelling}; }}"
            ),
            "tracked" => {
                format!("void subject(void) {{ struct S *o = q_alloc(); {spelling}; q_free(o); }}")
            }
            "unresolved" => format!("void subject(struct S *d) {{ q_free(d); {spelling}; }}"),
            other => panic!("unknown owner {other}"),
        };
        let run = gs(&body);
        if decision == "admit" {
            assert!(run.safe(), "{spelling} ({value}): {run:?}");
        } else {
            assert!(!run.safe(), "{spelling} ({value}): {run:?}");
        }
    }
}
