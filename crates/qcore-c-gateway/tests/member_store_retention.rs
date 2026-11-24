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

//! Card CG10-MEMBER-STORE-RETAINED-BY (gateway rule 70): a formal's pointer
//! stored into a member of another formal's object is a retention,
//! `retained_by(q, F.m…)`, and the body's summary states it.
//!
//! * the body: `s->p = q` (`F`, `q` formals never rebound, the member in
//!   `*F`, `F` untracked) is admitted and the body takes no custody of `q`;
//! * the summary: the body's validated contract (every formal `Borrowed`, a
//!   `None` result) with the path retention and every overwritten path,
//!   withheld from the function map and decided at each call by rule 12t;
//! * the callers: one that hands a static or null value at a path holder of
//!   its own formal discharges; one that keeps the record to its exit, that
//!   releases the retained object while held, or whose holder is not its own
//!   formal's path stays non-`Safe`.
//!
//! Every `Safe` here is `Safe` over the synthetic `q_*` fixture contracts and
//! promotes nothing.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const S: &str = "struct I { void *p; int x; };\n\
struct S { int a; unsigned bf : 3; void *p; struct I in; void (*cb)(void); struct S *self; \
void *slots[4]; };\nstruct C { struct C *next; struct C *prev; int n; };\nvoid f(void);\n";

const IND: &str = "indirect memory access is outside the resource projection";

fn run(source: &str) -> G {
    g(&format!("{S}{source}"))
}

fn run_function(source: &str, function: &str) -> G {
    g_function(&format!("{S}{source}"), function)
}

// ---------------------------------------------------------------------------
// The body
// ---------------------------------------------------------------------------

#[test]
fn a_formals_pointer_into_a_member_of_a_formals_object_is_read_as_retention() {
    if !clang_available() {
        return;
    }
    for body in [
        // Q36's example C, the shape rule 10v refused for want of an owner.
        "void subject(struct S *s, void *q) { s->p = q; }",
        "void subject(struct S *s, void *q) { s->in.p = q; }",
        // cJSON's `suffix_object`: each formal held by the other's member.
        "void subject(struct C *prev, struct C *item) { prev->next = item; item->prev = prev; }",
        // `rd_kafka_buf_set_maker`: beside a function pointer and a scalar.
        "void subject(struct S *s, void *q, void (*cb)(void)) { s->p = q; s->cb = cb; s->a = 1; }",
    ] {
        let run = run(body);
        assert!(run.safe(), "{body}: {run:?}");
    }
}

#[test]
fn a_store_outside_the_premises_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    for body in [
        // The retained formal is rebound or its address taken.
        "void subject(struct S *s, void *q) { q = s->p; s->p = q; }",
        "void subject(struct S *s, void *q) { void **a = &q; s->p = q; q_observe(a); }",
        // The holder's root is rebound.
        "void subject(struct S *s, struct S *t, void *q) { s = t; s->p = q; }",
        // The member is read through a second pointer, or is an element.
        "void subject(struct S *s, void *q) { s->self->p = q; }",
        "void subject(struct S *s, void *q, int i) { s->slots[i] = q; }",
        // A local's pointer (here an owned allocation: a custody move). Card
        // MEMBER-EFFECT-STORES (rule 38k) re-pins this deliberately: the bare
        // move `void *q = q_alloc(); s->p = q;` is now its hand-off
        // (`tests/member_effect_stores.rs`); used after the move it is refused.
        "void subject(struct S *s) { void *q = q_alloc(); s->p = q; q_free(q); }",
    ] {
        let run = run(body);
        assert!(!run.safe(), "{body}: {run:?}");
    }
    // The owner is the body's own tracked allocation: the member may own.
    let run = run(
        "void subject(void *q) { struct S *s = q_alloc(); if (s == 0) return; s->p = q; q_free(s); }",
    );
    assert!(!run.safe() && run.refused(IND), "{run:?}");
}

#[test]
fn a_body_that_takes_custody_of_the_retained_formal_is_not_safe() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(struct S *s, void *q) { s->p = q; q_free(q); }",
        "void subject(struct S *s, void *q) { q_free(q); s->p = q; }",
    ] {
        let run = run(body);
        assert!(!run.safe(), "{body}: {run:?}");
    }
}

// ---------------------------------------------------------------------------
// The summary and its callers
// ---------------------------------------------------------------------------

const KEEP: &str = "void keep(struct S *s, void *q) { s->p = q; }\n";
// The lexical parameter reader of the W4 summary reads a typedef-named
// function pointer formal (a declarator `void (*cb)(void)` publishes no summary).
const SET: &str = "typedef void (*cb_t)(void);\nvoid set_cb(struct S *s, void *q, cb_t cb) { s->p = q; s->cb = cb; }\n";

#[test]
fn a_caller_that_hands_a_static_or_null_value_to_its_own_formals_path_discharges() {
    if !clang_available() {
        return;
    }
    for caller in [
        "void subject(struct S *s) { keep(s, \"x\"); }",
        "void subject(struct S *s) { keep(s, 0); }",
        // The function-pointer subclass: a function designator adds no
        // retention, only the overwritten path.
        "void subject(struct S *s) { set_cb(s, 0, f); }",
        // A member path below the caller's formal (rule 12t's path holder).
        "void subject(struct S *s) { keep(s->self, 0); }",
    ] {
        let source = format!("{KEEP}{SET}{caller}");
        let run = run_function(&source, "subject");
        assert!(run.safe(), "{caller}: {run:?}");
    }
}

#[test]
fn a_caller_that_cannot_discharge_stays_not_safe() {
    if !clang_available() {
        return;
    }
    // Its own formal retained by its own formal's path: the record outlives
    // its frame (no retention is forwarded into its own summary).
    let source = format!("{KEEP}void subject(struct S *s, void *q) {{ keep(s, q); }}");
    let run = run_function(&source, "subject");
    assert!(!run.safe(), "{run:?}");
    assert!(run.has("retention-undecided"), "{run:?}");
    // A tracked object released while the holder keeps it.
    let source = format!(
        "{KEEP}void subject(struct S *s) {{ void *q = q_alloc(); if (q == 0) return; keep(s, q); q_free(q); }}"
    );
    let run = run_function(&source, "subject");
    assert!(!run.safe(), "{run:?}");
    assert!(run.has("retention-released-while-held"), "{run:?}");
    // A holder that is not the caller's own formal's path: the overwritten
    // member may be the caller's owning field, whatever value was stored.
    for caller in [
        "void subject(void) { struct S *s = q_alloc(); if (s == 0) return; keep(s, \"x\"); q_free(s); }",
        "void subject(void) { struct S *s = q_alloc(); if (s == 0) return; set_cb(s, 0, f); q_free(s); }",
        // The formal is rebound: its path is not the caller's slot.
        "void subject(struct S *s, struct S *t) { s = t; keep(s, 0); }",
    ] {
        let source = format!("{KEEP}{SET}{caller}");
        let run = run_function(&source, "subject");
        assert!(!run.safe(), "{caller}: {run:?}");
        assert!(run.has("retention-undecided"), "{caller}: {run:?}");
    }
}

#[test]
fn a_body_whose_stores_are_all_rule_10v_publishes_no_summary() {
    if !clang_available() {
        return;
    }
    // Rule 10v's gate is unchanged for a body with no rule-70 store.
    let source = "void init(struct S *s) { s->p = 0; s->a = 0; }\n\
                  void subject(struct S *s) { init(s); }";
    let run = run_function(source, "subject");
    assert!(!run.safe(), "{run:?}");
    assert!(
        run.refused("call to uncontracted function `init`"),
        "{run:?}"
    );
}
