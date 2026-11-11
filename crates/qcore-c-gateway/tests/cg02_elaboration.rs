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

//! Card CG02-ELABORATION-CORE (gateway rule 10s): route G's use of the
//! common elaborator, over the analysis's §8 axes (position, effect,
//! evaluation order, place), beyond CG00's flipped controls in
//! `clang_gap_phase1.rs`. Every admission here is a fallback after the older
//! routes refused; `Safe` is over the synthetic fixture contracts.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct S { int a; int b; int arr[4]; struct I { int x; } in; void *p; };\n";

fn gs(source: &str) -> G {
    g(&format!("{S}{source}"))
}

const LOOP_REFUSAL: &str = "unmodeled call, aggregate or indirect access in loop condition";

#[test]
fn positions_statement_return_and_condition() {
    if !clang_available() {
        return;
    }
    // Statement: a field increment and a compound on a mixed path.
    assert!(gs("void subject(struct S *s, int i) { s->arr[i]++; s->in.x |= 4; }").safe());
    // A path read inside another path's address (its obligations are the
    // outer path's plan).
    assert!(gs("void subject(struct S *s) { s->arr[s->a]++; }").safe());
    // Declaration: the old value of a field.
    assert!(gs("int subject(struct S *s) { int j = s->a--; return j; }").safe());
    // Return: an increment's value, a field's new value, an assignment.
    assert!(gs("int subject(int i) { return i++; }").safe());
    assert!(gs("int subject(struct S *s) { return ++s->a; }").safe());
    assert!(gs("int subject(int a) { int x; return (x = a + 1); }").safe());
    // Condition: `while (n-- > 0)`, `for (; i-- ;)`, `if (++k == 3)`.
    let run = gs("int subject(int n) { int c = 0; while (n-- > 0) c++; return c; }");
    assert!(run.safe(), "{run:?}");
    assert!(gs("int subject(int i) { int c = 0; for (; i--;) c++; return c; }").safe());
    assert!(gs("int subject(int k) { if (++k == 3) return 1; return 0; }").safe());
    assert!(gs("int subject(int n) { int c = 0; do c++; while (--n > 0); return c; }").safe());
}

#[test]
fn obligations_of_the_admitted_accesses_stand() {
    if !clang_available() {
        return;
    }
    // A field write through a released object.
    let run = gs("void subject(void) { struct S *s = q_alloc(); q_free(s); s->a++; }");
    assert!(!run.safe(), "{run:?}");
    // A condition increment through a released object.
    let run = gs("int subject(void) { struct S *s = q_alloc(); q_free(s); \
                  while (s->a-- > 0) {} return 0; }");
    assert!(!run.safe(), "{run:?}");
    // A leak survives a comma statement.
    let run = gs("void subject(void) { void *p = q_alloc(); q_observe(p), q_observe(p); }");
    assert!(!run.safe(), "{run:?}");
}

#[test]
fn refusals_stay_byte_for_byte() {
    if !clang_available() {
        return;
    }
    // Flipped by CG04-CONDITION-ACQUISITION (rule 10x): an increment in a
    // call-free leaf of a connective condition is this elaborator's, leaf by
    // leaf, sequenced by `&&`; the connective itself is the desugaring's edge,
    // not a value `&&` (CG06 keeps value connectives outside conditions).
    let run = gs("int subject(int n, int m) { int c = 0; while (n-- > 0 && m) c++; return c; }");
    assert!(!run.refused(LOOP_REFUSAL), "{run:?}");
    assert!(run.safe(), "{run:?}");
    // A connective below one leaf that increments stays refused.
    let run = gs("int subject(int n, int m) { int c = 0; while ((n-- && m) > 0) c++; return c; }");
    assert!(run.refused(LOOP_REFUSAL), "{run:?}");
    // A call beside an increment in a condition (CG04/CG05).
    let run =
        gs("int subject(void *p) { int n = 3; while (r_borrow_code(p) + n-- > 0) {} return n; }");
    assert!(!run.safe(), "{run:?}");
    // A pointer-valued store in an increment's value position (CG11).
    let run = gs("void subject(struct S *s, void *v) { void *w; w = (s->p = v); }");
    assert!(!run.safe(), "{run:?}");
    // An unsequenced pair on one field (C undefined behaviour).
    let run = gs("void subject(struct S *s) { s->a = s->a++; }");
    assert!(!run.safe(), "{run:?}");
    // Volatile (CG15).
    let run = gs("void subject(volatile int *v) { int j = 0; j = (*v)++; }");
    assert!(!run.safe(), "{run:?}");
}

#[test]
fn sequenced_pieces_keep_their_contracts() {
    if !clang_available() {
        return;
    }
    // Each comma operand is the classifier's own statement, in order.
    let run =
        gs("void subject(void) { void *p; p = (q_observe(q_alloc()), q_alloc()); q_free(p); }");
    assert!(!run.safe(), "{run:?}");
    let run = gs("void subject(void) { void *p; int k = 0; p = (k++, q_alloc()); q_free(p); }");
    assert!(run.safe(), "{run:?}");
    // A chain through a declared pointer: `q = q_alloc()` then `p = q`.
    let run = gs("void subject(void) { void *p; void *q; p = q = q_alloc(); q_free(q); }");
    assert!(run.safe(), "{run:?}");
    // The chain's result is the stored object: releasing it twice is caught.
    let run =
        gs("void subject(void) { void *p; void *q; p = q = q_alloc(); q_free(q); q_free(p); }");
    assert!(!run.safe(), "{run:?}");
}
