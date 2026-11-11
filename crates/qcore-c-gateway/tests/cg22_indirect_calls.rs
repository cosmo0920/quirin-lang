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

//! Card CG22-INDIRECT-CALLS-AND-CALLBACKS (gateway rule 11p): route G's
//! candidate callee sets, end to end over the CG00 fixture contracts
//! (synthetic `q_*` assumptions, not library facts). Every admission is
//! paired with a refusal control.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

fn uncontracted(callee: &str) -> String {
    format!("call to uncontracted function `{callee}`")
}

#[test]
fn a_local_pointer_with_one_definition_is_its_candidate() {
    if !clang_available() {
        return;
    }
    let run = g("typedef void (*cb_t)(void *); \
                 void subject(void) { void *p = q_alloc(); cb_t f = q_free; f(p); }");
    assert!(run.safe(), "{run:?}");
    assert!(run.calls.iter().any(|call| call == "q_free"), "{run:?}");
    // The same through `&f` and `(*f)`.
    let run = g("typedef void (*cb_t)(void *); \
                 void subject(void) { void *p = q_alloc(); cb_t f = &q_free; (*f)(p); }");
    assert!(run.safe(), "{run:?}");
    // Two definitions: the branch over both candidates.
    let run = g("typedef void (*cb_t)(void *); \
                 void subject(int c) { void *p = q_alloc(); cb_t f = q_free; \
                 if (c) f = q_nullable_free; f(p); }");
    assert!(run.safe(), "{run:?}");
    // The declaration of a local with an inline function-pointer declarator
    // is refused by the statement classifier before any call is read
    // (`CG22-INLINE-FNPTR-LOCAL-DECLARATION`): the call is resolved, the
    // declaration is not.
    let run = g("void subject(void) { void *p = q_alloc(); void (*f)(void *) = q_free; f(p); }");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(&uncontracted("void")), "{run:?}");
    assert!(run.calls.iter().any(|call| call == "q_free"), "{run:?}");
}

#[test]
fn a_dereferenced_designator_is_the_direct_call() {
    if !clang_available() {
        return;
    }
    let run = g("void subject(void) { void *p = q_alloc(); (*q_free)(p); }");
    assert!(run.safe(), "{run:?}");
    // The double release through the same spelling is still found.
    let run = g("void subject(void) { void *p = q_alloc(); (*q_free)(p); (*q_free)(p); }");
    assert!(run.unsafe_(), "{run:?}");
}

#[test]
fn a_two_candidate_table_both_safe_is_safe() {
    if !clang_available() {
        return;
    }
    let run = g(
        "static void (*const T[2])(void *) = { q_free, q_nullable_free }; \
                 void subject(int i) { void *p = q_alloc(); T[i](p); }",
    );
    assert!(run.safe(), "{run:?}");
    // A constant record table's member.
    let run = g(
        "struct ops { void (*release)(void *); void (*observe)(void *); }; \
                 static const struct ops OPS = { q_free, q_observe }; \
                 void subject(void) { void *p = q_alloc(); OPS.observe(p); OPS.release(p); }",
    );
    assert!(run.safe(), "{run:?}");
}

#[test]
fn the_join_over_candidates_keeps_the_worst() {
    if !clang_available() {
        return;
    }
    // One candidate releases, the other only observes: its arm leaks.
    let run = g(
        "static void (*const T[2])(void *) = { q_free, q_observe }; \
                 void subject(int i) { void *p = q_alloc(); T[i](p); }",
    );
    assert!(!run.safe(), "{run:?}");
    assert!(run.unsafe_(), "{run:?}");
}

#[test]
fn one_unknown_candidate_is_unknown() {
    if !clang_available() {
        return;
    }
    let run = g("void opaque_release(void *p); \
                 static void (*const T[2])(void *) = { q_free, opaque_release }; \
                 void subject(int i) { void *p = q_alloc(); T[i](p); }");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(&uncontracted("opaque_release")), "{run:?}");
}

#[test]
fn a_pointer_from_an_opaque_call_is_unknown() {
    if !clang_available() {
        return;
    }
    let run = g("typedef void (*cb_t)(void *); cb_t get_cb(void); \
                 void subject(void) { void *p = q_alloc(); cb_t f = get_cb(); f(p); }");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(&uncontracted("f")), "{run:?}");
}

#[test]
fn a_callback_context_released_before_invocation_is_not_safe() {
    if !clang_available() {
        return;
    }
    let run = g("typedef void (*cb_t)(void *); \
                 void subject(void) { void *p = q_alloc(); cb_t cb = q_observe; \
                 q_free(p); cb(p); }");
    assert!(!run.safe(), "{run:?}");
    // Exactly the verdict and leaves of the direct invocation.
    let direct = g("void subject(void) { void *p = q_alloc(); q_free(p); q_observe(p); }");
    assert_eq!(run.status, direct.status, "{run:?} {direct:?}");
    assert_eq!(run.diagnostics, direct.diagnostics);
    // Released through the resolved callback, then used: not Safe either.
    let run = g("typedef void (*cb_t)(void *); \
                 void subject(void) { void *p = q_alloc(); cb_t done = q_free; \
                 done(p); q_observe(p); }");
    assert!(!run.safe(), "{run:?}");
}

#[test]
fn open_sets_keep_their_older_refusal() {
    if !clang_available() {
        return;
    }
    // A formal's value is the caller's.
    let run = g("void subject(void (*fp)(void *)) { void *p = q_alloc(); fp(p); q_free(p); }");
    assert!(run.refused(&uncontracted("fp")), "{run:?}");
    // A member read through a pointer (vtable-like).
    let run = g("struct V { void (*cb)(void *); }; \
                 void subject(struct V *v) { void *p = q_alloc(); v->cb(p); q_free(p); }");
    assert!(run.refused(IND), "{run:?}");
    // An address-taken local.
    let run = g("typedef void (*cb_t)(void *); void reset(cb_t *f); \
                 void subject(void) { void *p = q_alloc(); cb_t f = q_free; \
                 reset(&f); f(p); }");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(&uncontracted("f")), "{run:?}");
    // A local with no initializer.
    let run = g("typedef void (*cb_t)(void *); \
                 void subject(int c) { void *p = q_alloc(); cb_t f; \
                 if (c) f = q_free; f(p); }");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(&uncontracted("f")), "{run:?}");
    // A mutable static table.
    let run = g("static void (*T[1])(void *) = { q_free }; \
                 void subject(void) { void *p = q_alloc(); T[0](p); }");
    assert!(run.unknown(), "{run:?}");
    // An explicit cast on the way.
    let run = g("typedef void (*cb_t)(void *); \
                 void subject(void) { void *p = q_alloc(); cb_t f = (cb_t)q_free; f(p); }");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(&uncontracted("f")), "{run:?}");
    // A null definition.
    let run = g("typedef void (*cb_t)(void *); \
                 void subject(int c) { void *p = q_alloc(); cb_t f = 0; \
                 if (c) f = q_free; f(p); }");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(&uncontracted("f")), "{run:?}");
}

#[test]
fn a_shadowed_candidate_is_not_rewritten() {
    if !clang_available() {
        return;
    }
    let run = g("static void (*const T[1])(void *) = { q_free }; \
                 void subject(void) { void *p = q_alloc(); int q_free = 0; T[0](p); \
                 (void)q_free; }");
    assert!(!run.safe(), "{run:?}");
}

#[test]
fn several_candidates_outside_a_statement_stay_refused() {
    if !clang_available() {
        return;
    }
    let run = g(
        "static int (*const T[2])(void *) = { r_release_code, r_consume_code }; \
                 int subject(int i) { void *p = q_alloc(); if (T[i](p)) return 1; return 0; }",
    );
    assert!(run.unknown(), "{run:?}");
}
