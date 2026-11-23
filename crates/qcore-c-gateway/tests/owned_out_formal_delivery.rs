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

//! OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY (gateway rule 74l): an owned
//! result stored through a formal's slot (`*out = make();`)
//! (`docs/reports/owned-constructor-out-delivery/`).
//!
//! The bodies are held-out fixtures of the census shapes over the synthetic
//! `q_*` API, through the whole Clang route. They are not claims about any
//! upstream body. Change pins (failing at the base, which read every one of
//! these stores as `discarded owned result`) are marked; the rest pin the
//! verdicts the card keeps.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

fn slot_refusal(callee: &str, slot: &str) -> String {
    format!(
        "owned result of resource API `{callee}` stored through `{slot}` without an out-parameter contract"
    )
}

fn discard(callee: &str) -> String {
    format!("discarded owned result of resource API `{callee}`")
}

fn uncontracted(callee: &str) -> String {
    format!("call to uncontracted function `{callee}`")
}

/// The sound counterpart: a store on every exit is W4's `Out` summary, the
/// caller takes custody of what it delivers.
#[test]
fn an_every_exit_delivery_is_the_callers_custody() {
    if !clang_available() {
        return;
    }
    const FILL: &str = "void fill(void **out) { *out = q_alloc(); }";
    assert!(g_function(FILL, "fill").safe());
    let kept = g(&format!(
        "{FILL} void subject(void) {{ void *p; fill(&p); q_free(p); }}"
    ));
    assert!(kept.safe(), "{kept:?}");
    // A caller that ignores the delivered object leaks in the caller.
    let ignored = g(&format!("{FILL} void subject(void) {{ void *p; fill(&p); }}"));
    assert!(ignored.unsafe_() && ignored.has("Leak"), "{ignored:?}");
}

/// CHANGE PIN. `rd_kafka_assignment` / `rd_kafka_subscription`
/// (rdkafka_subscription.c:337, :365): the slot first takes another object's
/// member, then the fresh list only when that is NULL and `err` is 0, and the
/// early returns leave it unwritten.
#[test]
fn the_assignment_and_subscription_shape_is_refused_by_name() {
    if !clang_available() {
        return;
    }
    for slot in ["partitions", "topics"] {
        let source = format!(
            "struct H {{ void *m; }}; \
             int subject(struct H *h, void **{slot}, int err) {{ \
               if (!h) return -1; \
               *{slot} = h->m; h->m = 0; \
               if (!*{slot} && !err) {{ *{slot} = q_alloc(); }} \
               return err; }}"
        );
        let run = g(&source);
        assert!(run.unknown(), "{run:?}");
        assert!(
            run.refused(&slot_refusal("q_alloc", &format!("*{slot}"))),
            "{run:?}"
        );
        assert!(!run.refused(&discard("q_alloc")), "{run:?}");
    }
}

/// CHANGE PIN. `rd_kafka_handle_OffsetFetch` (rdkafka_request.c:1223): an
/// in-out slot that keeps the caller's object when it has one.
#[test]
fn the_offset_fetch_in_out_slot_is_refused_by_name() {
    if !clang_available() {
        return;
    }
    let run = g("void subject(void **offsets) { if (!*offsets) *offsets = q_alloc(); }");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(&slot_refusal("q_alloc", "*offsets")), "{run:?}");
    assert!(!run.refused(&discard("q_alloc")), "{run:?}");
}

/// `ut_create_topic_partition_lists` (rdkafka_sticky_assignor.c:2235) stores
/// into an element (`lists[i] = …`), not through a `*NAME` slot: out of this
/// rule, its refusal is unchanged (CG27's c06 shape).
#[test]
fn an_element_store_keeps_its_refusal() {
    if !clang_available() {
        return;
    }
    let run = g("void subject(void) { void *a[4]; int i; \
                 for (i = 0; i < 4; i++) a[i] = q_alloc(); \
                 for (i = 0; i < 4; i++) q_free(a[i]); }");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(&discard("q_alloc")), "{run:?}");
}

/// CHANGE PIN (text); the verdict is kept. A partial-path store publishes no
/// summary, so its caller keeps the uncontracted call.
#[test]
fn a_partial_path_store_stays_refused() {
    if !clang_available() {
        return;
    }
    const FILL: &str = "int fill(void **out, int c) { if (c) *out = q_alloc(); return c; }";
    let run = g_function(FILL, "fill");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(&slot_refusal("q_alloc", "*out")), "{run:?}");
    let caller = g(&format!(
        "{FILL} void subject(void) {{ void *p = 0; fill(&p, 1); }}"
    ));
    assert!(caller.unknown(), "{caller:?}");
    assert!(caller.refused(&uncontracted("fill")), "{caller:?}");
}

/// CHANGE PIN (text); the verdict is kept. A store followed by a release, or
/// by a second store, delivers no live object: refused, no summary.
#[test]
fn a_store_followed_by_a_release_or_a_second_store_stays_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "void fill(void **out) { *out = q_alloc(); q_free(*out); }",
        "void fill(void **out) { *out = q_alloc(); *out = q_alloc(); }",
    ] {
        let run = g_function(body, "fill");
        assert!(run.unknown(), "{body}: {run:?}");
        assert!(run.refused(&slot_refusal("q_alloc", "*out")), "{body}: {run:?}");
        let caller = g(&format!(
            "{body} void subject(void) {{ void *p; fill(&p); }}"
        ));
        assert!(caller.refused(&uncontracted("fill")), "{body}: {caller:?}");
    }
    // The variable's store keeps its own sibling refusal.
    let run = g_function(
        "void fill(void **out) { void *p = q_alloc(); *out = p; q_free(p); }",
        "fill",
    );
    assert!(run.unknown(), "{run:?}");
    assert!(
        run.refused("store through `*out` without an out-parameter contract"),
        "{run:?}"
    );
}

/// CHANGE PIN (text); the verdict is kept. A possibly-null result is no
/// delivery the caller is told about.
#[test]
fn a_possibly_null_result_stays_refused() {
    if !clang_available() {
        return;
    }
    let run = g_function("void fill(void **out) { *out = q_maybe_alloc(); }", "fill");
    assert!(run.unknown(), "{run:?}");
    assert!(
        run.refused(&slot_refusal("q_maybe_alloc", "*out")),
        "{run:?}"
    );
}

/// CHANGE PIN (text); the verdict is kept. A slot of this frame's own pointer
/// is no caller's: no summary can deliver through it.
#[test]
fn a_slot_the_caller_cannot_see_stays_refused() {
    if !clang_available() {
        return;
    }
    let run = g("void subject(void) { void *slot; void **p = &slot; *p = q_alloc(); q_free(slot); }");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(&slot_refusal("q_alloc", "*p")), "{run:?}");
}

/// A real discard keeps its text.
#[test]
fn a_real_discard_keeps_its_text() {
    if !clang_available() {
        return;
    }
    let run = g("void subject(void) { q_alloc(); }");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(&discard("q_alloc")), "{run:?}");
}
