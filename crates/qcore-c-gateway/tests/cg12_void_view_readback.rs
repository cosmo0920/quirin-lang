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

//! Card CG12-VOID-VIEW-READBACK (gateway rule 11j): `void *` views in typed
//! paths and the readback of an identity stored in a `void *` member.
//!
//! The shared vectors of `src/void_view.rs` (`POSITION_VECTORS`,
//! `DECISION_VECTORS`, `EVENT_VECTORS`; the same rows as
//! `proof/QCore/VoidViewReadback.lean`) are read from its text and run
//! through the gateway here, with the refusal controls the card names.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct S { int a; int b; void *p; void *v; struct S *next; }; \
struct T { int x; char *name; }; struct A { int y; };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

/// The rows of the table `name` in `src/void_view.rs`, `arity` literals each
/// (string literals and `true`/`false`, in order).
fn table(name: &str, arity: usize) -> Vec<Vec<String>> {
    let text = include_str!("../src/void_view.rs");
    let start = text
        .find(&format!("pub(crate) const {name}:"))
        .unwrap_or_else(|| panic!("{name}"));
    let body = &text[start..];
    let body = &body[body.find("= &[").expect("table start") + 4..];
    let body = &body[..body.find("\n];").expect("table end")];
    let mut fields = Vec::new();
    let mut chars = body.chars().peekable();
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
            't' | 'f' => {
                let mut word = c.to_string();
                while chars.peek().is_some_and(char::is_ascii_alphabetic) {
                    word.push(chars.next().expect("letter"));
                }
                if word == "true" || word == "false" {
                    fields.push(word);
                }
            }
            _ => {}
        }
    }
    assert_eq!(fields.len() % arity, 0, "{name}: {fields:?}");
    fields.chunks(arity).map(<[String]>::to_vec).collect()
}

// ---------------------------------------------------------------------------
// The view
// ---------------------------------------------------------------------------

#[test]
fn every_position_vector_is_the_route_outcome() {
    if !clang_available() {
        return;
    }
    let rows = table("POSITION_VECTORS", 5);
    assert_eq!(rows.len(), 8);
    for row in rows {
        let (spelling, position, never_bound, tracked, admitted) = (
            &row[0],
            &row[1],
            row[2] == "true",
            row[3] == "true",
            row[4] == "true",
        );
        let root = spelling
            .trim_start_matches(['*', '(', ' '])
            .split(')')
            .nth(1)
            .and_then(|rest| rest.trim_start().chars().next())
            .expect("root");
        let body = match (root, never_bound, tracked) {
            ('a' | 'f', true, false) => {
                format!("void subject(void *a, struct S *f) {{ int x = {spelling}; cg_sink(x); }}")
            }
            // Bound later in the body (not never bound), untracked at the
            // access: rule 11d's premise.
            ('o', false, false) => {
                format!("void subject(void *o) {{ int x = {spelling}; cg_sink(x); o = 0; }}")
            }
            ('o' | 'h', false, true) => format!(
                "void subject(void) {{ void *{root} = q_alloc(); int x = {spelling}; cg_sink(x); \
                 q_free({root}); }}"
            ),
            ('s', false, true) => format!(
                "void subject(void) {{ struct S *s = q_alloc(); struct T *t = q_alloc(); s->v = t; \
                 int x = {spelling}; cg_sink(x); q_free(t); q_free(s); }}"
            ),
            other => panic!("no body for {spelling}: {other:?}"),
        };
        let run = gs(&body);
        assert_eq!(run.safe(), admitted, "{spelling} ({position}): {run:?}");
    }
}

/// CG00's M05 corpus shapes, in every typed position: a view over a
/// never-bound formal in a statement, a condition, a return and a call
/// actual, and a scalar and a pointer store through one.
#[test]
fn views_of_never_bound_void_pointers_are_read_in_every_position() {
    if !clang_available() {
        return;
    }
    for body in [
        "int subject(const void *a) { return *(const int *)a; }",
        "int subject(const void *a) { return ((const struct T *)a)->x; }",
        "void subject(const void *a) { const char *n = *(const char **)a; q_observe((void *)n); }",
        "void subject(struct S *s) { *(int *)s->p = 1; }",
        "void subject(struct S *s) { int x = *(int *)s->p; cg_sink(x); }",
        "void subject(struct S *s) { if (*(int *)s->p) cg_sink(1); }",
        "void subject(const void *a) { cg_sink(((const struct T *)a)->x); }",
        "void subject(void *result) { **(int **) result = 1; }",
        "unsigned char subject(const void *src) { unsigned char c = ((const unsigned char *)src)[4]; return c; }",
    ] {
        let run = gs(body);
        assert!(run.safe(), "{body}: {run:?}");
    }
}

/// Unlisted views keep their refusal: a cast between object pointers (a
/// pun, M22), a view over a link of a tracked owner, a view over a call
/// result and a view over pointer arithmetic.
#[test]
fn unlisted_views_are_refused() {
    if !clang_available() {
        return;
    }
    // Card CAST-FORMALIZATION (rule 12h), a deliberate flip: `*(int *)p` of
    // an `int *p` is a qualifier conversion (the same type), read through as
    // the object itself with no view; it is Safe now and pinned in
    // `cast_formalization.rs`. The pun below keeps its refusal.
    for body in [
        "void subject(long *p) { int x = *(int *)p; cg_sink(x); }",
        "void subject(void) { struct S *s = q_alloc(); struct T *t = q_alloc(); s->v = t; \
         int x = ((struct T *) s->v)->x; cg_sink(x); q_free(t); q_free(s); }",
        "void *cg_get(void); void subject(void) { int x = *(int *)cg_get(); cg_sink(x); }",
        "void subject(struct S *s, int k) { int x = *(int *)((char *)s->p + k); cg_sink(x); }",
        // C02-P-ALIAS: a view of a `void *` root as `struct T *` where the
        // body declares another `struct T *` (a may-alias peer) is refused.
        "void subject(struct T *t, void *a) { q_free(t); int x = ((struct T *)a)->x; cg_sink(x); }",
    ] {
        let run = gs(body);
        assert!(!run.safe(), "{body}: {run:?}");
    }
}

// ---------------------------------------------------------------------------
// The readback
// ---------------------------------------------------------------------------

#[test]
fn every_decision_vector_is_the_route_outcome() {
    if !clang_available() {
        return;
    }
    let rows = table("DECISION_VECTORS", 6);
    assert_eq!(rows.len(), 7);
    for row in rows {
        let (statements, decision) = (&row[0], row[5].as_str());
        let run = gs(&format!("void subject(void) {{ {statements} }}"));
        match decision {
            "identity" => assert!(run.safe(), "{statements}: {run:?}"),
            "pun" => {
                assert!(!run.safe(), "{statements}: {run:?}");
                assert!(
                    run.refused_prefix("void * member read back as a type other than"),
                    "{statements}: {run:?}"
                );
            }
            _ => {
                assert!(!run.safe(), "{statements}: {run:?}");
                assert!(
                    !run.refused_prefix("void * member read back as a type other than"),
                    "{statements}: {run:?}"
                );
            }
        }
    }
}

#[test]
fn every_event_vector_is_the_route_outcome() {
    if !clang_available() {
        return;
    }
    let rows = table("EVENT_VECTORS", 2);
    assert_eq!(rows.len(), 6);
    for row in rows {
        let (events, yields) = (&row[0], row[1] == "true");
        let body = format!(
            "void subject(int n) {{ struct S *s = q_alloc(); struct T *t = q_alloc(); \
             struct T *u = q_alloc(); {events} struct T *c = s->v; q_free(c); q_free(s); }}"
        );
        let run = gs(&body);
        assert_eq!(run.safe(), yields, "{events}: {run:?}");
        if !yields {
            // No identity: the release through the readback resolves nothing.
            assert!(
                run.has("UnknownAlias { variable: \"c\" }"),
                "{events}: {run:?}"
            );
        }
    }
}

/// The card's refusal control: a store, then an uncontracted call taking the
/// owner, then a readback: the readback has no identity (unknown), whatever
/// else the call refuses.
#[test]
fn a_store_then_an_uncontracted_call_taking_the_owner_leaves_the_readback_unknown() {
    if !clang_available() {
        return;
    }
    let run = gs("void cg_opaque_take(struct S *s); \
         void subject(void) { struct S *s = q_alloc(); struct T *t = q_alloc(); s->v = t; \
         cg_opaque_take(s); struct T *c = s->v; q_free(c); q_free(s); }");
    assert!(!run.safe(), "{run:?}");
    assert!(
        run.refused("call to uncontracted function `cg_opaque_take`"),
        "{run:?}"
    );
    assert!(run.has("UnknownAlias { variable: \"c\" }"), "{run:?}");
}

/// The card's refusal control: reading back as a different struct type from
/// the one stored is type punning, refused by name.
#[test]
fn reading_back_as_another_struct_type_is_refused() {
    if !clang_available() {
        return;
    }
    for readback in [
        "struct A *c = (struct A *) s->v;",
        "struct A *c = s->v;",
        "struct A *c; c = (struct A *) s->v;",
    ] {
        let run = gs(&format!(
            "void subject(void) {{ struct S *s = q_alloc(); struct T *t = q_alloc(); s->v = t; \
             {readback} q_free(c); q_free(s); }}"
        ));
        assert!(!run.safe(), "{readback}: {run:?}");
        assert!(
            run.refused_prefix("void * member read back as a type other than"),
            "{readback}: {run:?}"
        );
    }
    // Two views of an identity whose stored type is unknown also disagree.
    let run = gs(
        "void subject(void) { struct S *s = q_alloc(); s->v = q_alloc(); \
         struct T *c = s->v; struct A *d = s->v; q_free(c); q_free(s); }",
    );
    assert!(!run.safe(), "{run:?}");
}

/// Where the identity cannot be established the read is the borrowed alias
/// it was (no identity, never guessed): a caller's owner, a formal
/// destination, a member written through an alias of the owner.
#[test]
fn a_readback_with_no_established_identity_is_a_borrowed_alias() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(struct S *s) { struct T *c = s->v; q_free(c); }",
        "void subject(struct T *c) { struct S *s = q_alloc(); struct T *t = q_alloc(); s->v = t; \
         c = s->v; q_free(t); q_free(s); q_free(c); }",
        "void subject(void) { struct S *s = q_alloc(); struct T *t = q_alloc(); s->v = t; \
         void *w = s; struct T *c = s->v; q_free(c); q_free(s); }",
    ] {
        let run = gs(body);
        assert!(!run.safe(), "{body}: {run:?}");
    }
}
