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

//! Card CG05-ELABORATOR-CALL-FOOTPRINTS (gateway rule 11n): the elaborator's
//! own call events carry their contract footprint, and a compound assignment
//! or store whose value holds a call is lowered through card CG03's one
//! application point. Every admission is paired with its fail-closed
//! controls; `Safe` is over the synthetic trusted fixture contracts of card
//! CG00.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const UNORDERED: &str = "unsequenced with another event (CG05)";
const EMBEDDED: &str = "resource call with a formal embedded in an unmodeled expression";

/// The brief's admission: `x += f(p)` with `f` releasing `p` and `x` a scalar
/// local whose address is never taken. The release is applied once.
#[test]
fn a_scalar_destination_read_commutes_with_a_releasing_call() {
    if !clang_available() {
        return;
    }
    let run = g("void subject(void) { int x = 0; void *p = q_alloc(); x += r_release_code(p); }");
    assert!(run.safe(), "{run:?}");
    assert!(
        run.calls.iter().any(|call| call == "r_release_code"),
        "{run:?}"
    );
    // The release is really applied: a second one is a double release.
    let run = g(
        "void subject(void) { int x = 0; void *p = q_alloc(); x += r_release_code(p); q_free(p); }",
    );
    assert!(!run.safe(), "{run:?}");
    // And without it the allocation leaks.
    let run = g("void subject(void) { int x = 0; void *p = q_alloc(); x += r_borrow_code(p); }");
    assert!(!run.safe(), "{run:?}");
    assert!(run.has("Leak") || run.has("leak"), "{run:?}");
    // Every compound operator, a borrowing call, a formal destination.
    for statement in [
        "x |= r_borrow_code(p);",
        "x -= r_borrow_code(p) + 1;",
        "x <<= r_borrow_code(p);",
        "x += r_borrow_code(p) + r_borrow_code(p);",
    ] {
        let run = g(&format!(
            "void subject(int x) {{ void *p = q_alloc(); {statement} q_free(p); }}"
        ));
        assert!(run.safe(), "{statement}: {run:?}");
    }
}

/// The brief's refusal: `*q += f(q)`: the destination is read through `q`,
/// unordered with a call that releases `q`'s identity.
#[test]
fn a_destination_read_through_the_released_identity_is_refused() {
    if !clang_available() {
        return;
    }
    // The elaborator's refusal (`… whose contract footprints do not commute`)
    // keeps the older route's refusal byte for byte; the unit test
    // `the_elaborator_reads_call_footprints_from_the_context` pins the reason.
    let run = g("void subject(void) { int *q = q_alloc(); *q += r_release_code(q); }");
    assert!(!run.safe(), "{run:?}");
    assert!(run.refused(EMBEDDED), "{run:?}");
    // Another identity that may alias: refused as well (names are never
    // proved distinct identities).
    let run = g("void subject(int *s) { void *q = q_alloc(); *s += r_release_code(q); }");
    assert!(!run.safe(), "{run:?}");
}

/// The brief's refusal: an uncontracted `g()` in `x += g()` has no footprint.
#[test]
fn an_uncontracted_call_stays_refused() {
    if !clang_available() {
        return;
    }
    let run = g("int subject(void) { int x = 0; x += opaque(); return x; }");
    assert!(!run.safe(), "{run:?}");
    let run =
        g("int undeclared(int); int subject(int k) { int x = 0; x += undeclared(k); return x; }");
    assert!(!run.safe(), "{run:?}");
    assert!(
        run.unsupported
            .iter()
            .any(|reason| reason.contains(UNORDERED))
            || run
                .unsupported
                .iter()
                .any(|reason| reason.contains("undeclared")),
        "{run:?}"
    );
}

/// A destination whose address is taken may be what an identity denotes: the
/// footprints never prove it distinct, so the pair stays refused.
#[test]
fn an_address_taken_destination_stays_refused() {
    if !clang_available() {
        return;
    }
    let run = g(
        "void subject(void) { int x = 0; void *p = q_alloc(); q_observe(&x); \
         x += r_release_code(p); }",
    );
    assert!(!run.safe(), "{run:?}");
    assert!(run.refused(EMBEDDED), "{run:?}");
}

/// The effects of an admitted call obey the acceptance: an owned result in a
/// value an operator consumes is refused (`Tested` must own nothing).
#[test]
fn the_acceptance_of_a_consumed_value_stands() {
    if !clang_available() {
        return;
    }
    let run = g("void subject(long x) { x += (long) q_alloc(); }");
    assert!(!run.safe(), "{run:?}");
    // Two releases of names that may alias, unordered: refused.
    let run = g(
        "void subject(void) { int x = 0; void *p = q_alloc(); void *q = q_alloc(); \
         x += r_release_code(p) + r_release_code(q); }",
    );
    assert!(!run.safe(), "{run:?}");
}
