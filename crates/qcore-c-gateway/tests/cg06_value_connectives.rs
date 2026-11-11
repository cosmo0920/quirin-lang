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

//! Card CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (gateway rule 10z): value
//! `&&`, `||`, `?:` and GNU `?:` in statements, declarations, stores, returns
//! and call actuals on route G, lowered by branching on each region's
//! deciding operand. `Safe` is over the synthetic CG00 fixture contracts.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct S { int a; int b; unsigned bf : 3; void *p; void *v; \
struct I { void *p; int x; } in; int arr[4]; void *slots[4]; void (*cb)(void); };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

/// The admitted positions: a declaration, a scalar and a member store, an
/// actual, a compound assignment, a return, a place base, a discarded value,
/// and a chain CG02 splits.
#[test]
fn value_regions_are_lowered_in_every_position() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(int a, int b) { int ok = a && b; return ok; }",
        "int subject(int a, int b) { int ok = 0; ok = a || b; return ok; }",
        "void subject(struct S *s, int n) { s->a = n < s->b ? n : s->b; }",
        "void subject(struct S *s, int n) { int m = s ? s->a : n; }",
        "void subject(struct S *s) { s->a += s->b > 0 ? 1 : 0; }",
        "void subject(struct S *s) { cg_log(\"%p\", s ? s->p : 0); }",
        "void subject(struct S *s, int n) { cg_log(\"%s\", n == 1 ? \"a\" : (n == 2 ? \"b\" : \"c\")); }",
        "int subject(void *p) { return r_borrow_code(p) ? 1 : 0; }",
        "void *subject(struct S *s) { return s->p ? (char *)s->p : 0; }",
        "void *subject(struct S *s) { return s ? s->p : 0; }",
        "void subject(struct S *s, int c) { (c ? s : s)->a = 1; }",
        "void subject(int c) { void *p = q_alloc(); c ? r_release_code(p) : r_consume_code(p); }",
        "void subject(int c) { int y; int x; x = (y = c ? 1 : 2); }",
        "void subject(void *p, int cnt) { int x = cnt + (r_borrow_code(p) ? 1 : 0); }",
    ] {
        let run = gs(source);
        assert!(run.safe(), "{source}: {run:?}");
    }
}

/// The brief's refusal controls.
#[test]
fn a_release_on_one_edge_is_carried_only_there() {
    if !clang_available() {
        return;
    }
    // `x = c && release(p)`: the release happens only where `c` is non-zero,
    // so the zero edge still owns `p` and leaks it.
    let run = gs("void subject(int c) { void *p = q_alloc(); int x = c && r_release_code(p); }");
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
    let run = gs("void subject(int c) { void *p = q_alloc(); c && r_release_code(p); }");
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
    // `x = p && release(p)`: the zero edge is `p == NULL`, which owns nothing.
    let run = gs("void subject(void) { void *p = q_alloc(); int x = p && r_release_code(p); }");
    assert!(run.safe(), "{run:?}");
}

#[test]
fn two_owned_identities_merge_to_one_of_and_a_release_refuses() {
    if !clang_available() {
        return;
    }
    // `r = c ? a : b; free(r)`: `r` may name either identity after the join;
    // the release through it is refused, never resolved to one of them.
    let run = gs(
        "void subject(int c) { void *a = q_alloc(); void *b = q_alloc(); void *r = c ? a : b; q_free(r); }",
    );
    assert!(!run.safe(), "{run:?}");
    assert!(run.has("UnknownAlias { variable: \"r\" }"), "{run:?}");
    // Carrying both obligations: each edge releases its own identity, and
    // the other stays owned there.
    let run = gs(
        "void subject(int c) { void *a = q_alloc(); void *b = q_alloc(); c ? r_release_code(a) : r_release_code(b); }",
    );
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
    let run = gs(
        "void subject(int c) { void *a = q_alloc(); void *b = q_alloc(); c ? r_release_code(a) : r_release_code(b); q_free(c ? b : a); }",
    );
    assert!(!run.safe(), "{run:?}");
}

#[test]
fn the_omitted_middle_operand_is_evaluated_once_and_f_only_on_zero() {
    if !clang_available() {
        return;
    }
    // `x ?: f()`: `f` runs only where `x` is zero. Were it evaluated on
    // every edge, `p` would be released everywhere and this would be Safe.
    let run = gs("void subject(int x) { void *p = q_alloc(); int r = x ?: r_release_code(p); }");
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
    // On the zero edge the release happens, so a later release is a double
    // release there (a violation of the synthetic fixture itself).
    let run = gs(
        "void subject(int x) { void *p = q_alloc(); int r = x ?: r_release_code(p); q_free(p); }",
    );
    assert!(run.unsafe_() && run.has("DoubleRelease"), "{run:?}");
    let run = gs(
        "void subject(int x) { void *p = q_alloc(); int r = x ?: r_borrow_code(p); q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
    // An `X` with an effect is evaluated once by C; the re-read lowering
    // would evaluate it twice, so it is refused.
    let run = gs(
        "void subject(int n) { void *p = q_alloc(); int r = n++ ?: r_borrow_code(p); q_free(p); }",
    );
    assert!(!run.safe(), "{run:?}");
}

/// Premises that keep a refusal.
#[test]
fn order_positions_and_cardinality_refuse() {
    if !clang_available() {
        return;
    }
    // A sibling with an effect beside the region (the increment's own
    // refusal stays).
    let run = gs("void subject(int n, int c) { int x = n++ + (c ? 1 : 2); }");
    assert!(!run.safe(), "{run:?}");
    // A region inside a statement expression (CG07).
    let run = gs("void subject(int c) { cg_sink(({ int t = c ? 1 : 2; t; })); }");
    assert!(!run.safe(), "{run:?}");
    // An unparenthesised comma middle operand would re-parse differently
    // once substituted; the parenthesised one is CG02's comma on its edge.
    let run = gs("void subject(int c, int a, int b) { int x = c ? a, b : 0; }");
    assert!(!run.safe(), "{run:?}");
    let run = gs("void subject(int c, int a, int b) { int x = c ? (a, b) : 0; }");
    assert!(run.safe(), "{run:?}");
    // An uncontracted call on one edge.
    let run =
        gs("void subject(int c) { void *p = q_alloc(); cg_sink(c ? opaque() : 0); q_free(p); }");
    assert!(!run.safe(), "{run:?}");
    // A connective in a controlling expression stays the condition CFG's.
    let run = gs(
        "void subject(int flag) { void *p = q_alloc(); if (flag ? r_release_code(p) : r_consume_code(p)) { } }",
    );
    assert!(run.safe(), "{run:?}");
}

/// The mechanical binding of `mergeVectors` in
/// `proof/QCore/ValueConnectives.lean`.
#[test]
fn lean_merge_vectors_run_through_route_g() {
    if !clang_available() {
        return;
    }
    let lean = include_str!("../../../proof/QCore/ValueConnectives.lean");
    let body = lean
        .split("def mergeVectors : List (String × Bool) := [")
        .nth(1)
        .unwrap();
    let body = &body[..body.find("\n]").unwrap()];
    let mut count = 0;
    for row in body.split("\n  (\"").skip(1) {
        let (source, rest) = row.split_once('"').unwrap();
        let expected = rest.contains("true");
        let run = gs(source);
        assert_eq!(run.safe(), expected, "{source}: {run:?}");
        count += 1;
    }
    assert_eq!(count, 5);
}
