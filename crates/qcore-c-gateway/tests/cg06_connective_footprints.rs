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

//! Card CG06-VALUE-CONNECTIVE-RESIDUALS, sibling-effect part (gateway rule
//! 11m): a value region whose sibling carries a call, composed with CG05's
//! contract footprints and lowered by rule 10z's branching on route G.
//! `Safe` is over the synthetic CG00 fixture contracts.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

/// The brief's admitted control, `log(ctx, p ? strlen(p) : 0,
/// release_and_status(q))` with identities the footprints prove disjoint:
/// the released identity is the fresh result of `r_wrap`, which no name
/// denotes. Each edge is CG05's nested-call statement.
#[test]
fn a_sibling_releasing_a_disjoint_identity_is_composed() {
    if !clang_available() {
        return;
    }
    let run = g(
        "void subject(void *p) { cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, r_release_code(r_wrap(p))); }",
    );
    assert!(run.safe(), "{run:?}");
    // A sibling that only borrows.
    let run =
        g("void subject(void *p) { cg_log(\"%s %d\", p ? \"a\" : \"b\", r_borrow_code(p)); }");
    assert!(run.safe(), "{run:?}");
    // The composed release still happens on both edges: the fresh identity
    // is released whichever edge runs, and an owned `p` beside it is still
    // owned at the exit (the zero edge is `p == NULL`, which owns nothing;
    // the non-zero edge only borrows `p`).
    let run = g(
        "void subject(void) { void *p = q_alloc(); cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, r_release_code(r_wrap(p))); }",
    );
    assert!(run.unsafe_() && run.has("Leak"), "{run:?}");
}

/// The brief's refusal controls: the same with the released identity `p`
/// itself, with another name (two names are never proved distinct
/// identities: a correction to the brief's "disjoint identities" control,
/// which CG05's cells cannot admit for two names), and with an uncontracted
/// sibling call. Each keeps the statement's old refusal, byte for byte.
#[test]
fn an_overlapping_or_uncontracted_sibling_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(void *p) { cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, r_release_code(p)); }",
        "void subject(void *p) { void *q = q_alloc(); cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, r_release_code(q)); }",
        "void subject(void *p) { cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, opaque()); }",
        // A releasing arm beside a sibling that reads the released identity.
        "void subject(void *p) { cg_log(\"%d %d\", p ? r_release_code(p) : 0, r_borrow_code(p)); }",
    ] {
        let run = g(source);
        assert!(!run.safe(), "{source}: {run:?}");
        assert!(
            !run.unsafe_(),
            "a refusal, never a verdict: {source}: {run:?}"
        );
    }
}

/// A sibling whose effect is not a call, and a releasing deciding call beside
/// a sibling that reads an identity (rule 10z's "deciding call beside a
/// non-inert sibling", whose footprints overlap), stay refused.
#[test]
fn a_non_call_sibling_and_an_overlapping_decision_stay_refused() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(void *p, int n) { cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, n++); }",
        "void subject(void *p, void **g) { cg_log(\"%d %p\", r_release_code(p) ? 1 : 0, *g); }",
    ] {
        let run = g(source);
        assert!(!run.safe(), "{source}: {run:?}");
    }
}
