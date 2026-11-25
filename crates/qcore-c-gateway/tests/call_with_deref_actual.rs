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

//! Card CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION (gateway rule 89): a call
//! statement whose first actual is `*NAME` is not a function-pointer
//! declarator `T (*NAME)`. Every probed shape is pinned with its verdict and
//! refusal; the base verdict is in each comment. Conventions as in
//! `clang_gap_phase1.rs`: `Safe` here is over the synthetic `q_*`/`cg_*`
//! fixture assumptions and promotes nothing.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const SHADOW: &str = "local declaration may shadow a formal resource identity";

/// Run one shape, never through `g`'s panic for a Clang rejection silently.
fn run(source: &str) -> G {
    g(source)
}

#[test]
fn a_call_with_a_dereferenced_first_actual_is_not_a_declarator() {
    if !clang_available() {
        return;
    }
    // Base: Unknown (SHADOW). Tip: Safe. The body's only statement is a call
    // to a borrowing contract; `q_observe((*out))` was already Safe at base
    // with the same obligations.
    for source in [
        "void subject(void **out) { q_observe(*out); }",
        "void subject(void **out, int x) { cg_pair(*out, x); }",
        "void subject(void **out, int x) { if (x) q_observe(*out); }",
    ] {
        let tip = run(source);
        assert!(tip.safe(), "{source}: {tip:?}");
    }
    // Unchanged Safe at base and tip: no `IDENT ( *` prefix.
    for source in [
        "void subject(void **out) { q_observe((*out)); }",
        "int subject(void **out) { int r; r = r_borrow_code(*out); return r; }",
    ] {
        assert!(run(source).safe(), "{source}");
    }
}

#[test]
fn the_admitted_call_keeps_its_own_reading() {
    if !clang_available() {
        return;
    }
    // Base: Unknown (SHADOW). Tip: Unknown for the call's own reason.
    let uncontracted = run("void undeclared_sink(void *p);\n\
         void subject(void **out) { undeclared_sink(*out); }");
    assert!(uncontracted.unknown(), "{uncontracted:?}");
    assert!(!uncontracted.refused(SHADOW), "{uncontracted:?}");
    assert!(
        uncontracted.refused("call to uncontracted function `undeclared_sink`"),
        "{uncontracted:?}"
    );
    // `**out`, and an owning store through `*list` (the minimised
    // rd_kafka_broker_share_session_add_remove_toppar), reach the access gate.
    for source in [
        "void subject(void ***out) { q_observe(**out); }",
        "void subject(void **list, void *item) { q_store(*list, item); }",
    ] {
        let tip = run(source);
        assert!(tip.unknown(), "{source}: {tip:?}");
        assert!(!tip.refused(SHADOW), "{source}: {tip:?}");
        assert!(tip.refused(IND), "{source}: {tip:?}");
    }
    let toppar = run("void subject(void **list, void *item) \
         { if (!*list) *list = q_alloc(); q_store(*list, item); }");
    assert!(toppar.unknown(), "{toppar:?}");
    assert!(!toppar.refused(SHADOW), "{toppar:?}");
    assert!(toppar.refused(IND), "{toppar:?}");
    assert!(
        toppar.refused(
            "owned result of resource API `q_alloc` stored through `*list` without an \
             out-parameter contract"
        ),
        "{toppar:?}"
    );
    // Two equal calls on one line: Clang's nodes are told apart by
    // occurrence (no SHADOW), but the access gate then refuses the `*out`
    // actual, which the single statement, the same pair on two lines and the
    // parenthesised pair (Safe at base and tip) all pass. A completeness
    // limit of the actual's reading, fail-closed.
    let twin = run("void subject(void **out) { q_observe(*out); q_observe(*out); }");
    assert!(twin.unknown(), "{twin:?}");
    assert!(!twin.refused(SHADOW), "{twin:?}");
    assert!(twin.refused(IND), "{twin:?}");
    for source in [
        "void subject(void **out) { q_observe(*out);\n q_observe(*out); }",
        "void subject(void **out) { q_observe((*out)); q_observe((*out)); }",
    ] {
        assert!(run(source).safe(), "{source}");
    }
    // The leak beside the call: Unsafe at base already (with the SHADOW
    // leaf), Unsafe at tip without it (a provisional Unsafe over the
    // synthetic contracts, nothing upstream).
    let leak = run("void subject(void **out) { void *p = q_alloc(); q_observe(*out); }");
    assert!(leak.unsafe_(), "{leak:?}");
    assert!(!leak.refused(SHADOW), "{leak:?}");
}

#[test]
fn shapes_without_the_prefix_are_unchanged() {
    if !clang_available() {
        return;
    }
    // Base and tip: Unknown (IND). None starts `IDENT ( *` with a formal
    // the guard reads, so the rule is never asked.
    for source in [
        "void subject(void **out) { (void)q_observe(*out); }",
        "void subject(void (*cb)(void *), void **out) { cb(*out); }",
        "struct CB { void (*cb)(void *); };\n\
         void subject(struct CB *s, void **out) { s->cb(*out); }",
    ] {
        let tip = run(source);
        assert!(tip.unknown(), "{source}: {tip:?}");
        assert!(tip.refused(IND), "{source}: {tip:?}");
    }
}

#[test]
fn every_real_shadowing_declarator_stays_refused() {
    if !clang_available() {
        return;
    }
    // Base and tip: Unknown (SHADOW).
    for source in [
        "void subject(void *out) { { int (*out); } }",
        "void subject(void *out) { { void (*out)(void); } }",
        "typedef int T;\nvoid subject(void *out) { { T (*out)[4]; } }",
        "typedef void *T;\nvoid subject(void *out) { { T (*out); } }",
        "typedef void *T;\nvoid subject(void *out) { if (out) { T (*out); } }",
        // A macro spelling a declarator: Clang reads a `DeclStmt`.
        "#define PTR(n) int n\nvoid subject(void *out) { { PTR(*out); } }",
    ] {
        let tip = run(source);
        assert!(tip.unknown(), "{source}: {tip:?}");
        assert!(tip.refused(SHADOW), "{source}: {tip:?}");
    }
    // `__typeof__(x) (*out);` is refused by the access gate at base and tip
    // (the declaration-likeness test does not read a `__typeof__` prefix;
    // JUMPED-SHADOW-FORMAL-READINGS owns that gap). Pinned as refused by
    // either name so that card's move does not conflict.
    let typeof_ = run("void subject(void *out) { int x; { __typeof__(x) (*out); } }");
    assert!(typeof_.unknown(), "{typeof_:?}");
    assert!(
        typeof_.refused(SHADOW) || typeof_.refused(IND),
        "{typeof_:?}"
    );
}
