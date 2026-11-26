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

//! OUT-SLOT-CONDITIONAL-DELIVERY (gateway rule 85): the keep-or-deliver
//! summary of an in-out slot (`if (!*out) *out = make();`)
//! (`docs/reports/out-slot-conditional-delivery/`).
//!
//! The bodies are held-out fixtures of the census shapes over the synthetic
//! `q_*` API, through the whole Clang route. They are not claims about any
//! upstream body. CHANGE PINS failed at the base, whose callers kept `call to
//! uncontracted function`; the rest pin verdicts the card keeps.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

const KEEP: &str = "void fill(void **out) { if (!*out) *out = q_alloc(); }";

fn uncontracted(callee: &str) -> String {
    format!("call to uncontracted function `{callee}`")
}

fn slot_refusal(callee: &str, slot: &str) -> String {
    format!(
        "owned result of resource API `{callee}` stored through `{slot}` without an out-parameter contract"
    )
}

fn subject(callee: &str, body: &str) -> G {
    g(&format!("{callee} void subject(void) {{ {body} }}"))
}

/// CHANGE PIN. A caller that knows its slot null takes custody of the
/// delivered object: releasing it is `Safe`, ignoring it leaks in the caller.
#[test]
fn a_known_null_slot_receives_the_callers_object() {
    if !clang_available() {
        return;
    }
    let released = subject(KEEP, "void *p = 0; fill(&p); q_free(p);");
    assert!(released.safe(), "{released:?}");
    let ignored = subject(KEEP, "void *p = 0; fill(&p);");
    assert!(ignored.unsafe_() && ignored.has("Leak"), "{ignored:?}");
}

/// CHANGE PIN. A caller holding its own live object keeps it: the call mints
/// no second custody, so one release is `Safe`, none leaks the caller's own
/// object, and two are a double release.
#[test]
fn a_held_object_stays_the_callers_and_is_never_minted_twice() {
    if !clang_available() {
        return;
    }
    let once = subject(KEEP, "void *p = q_alloc(); fill(&p); q_free(p);");
    assert!(once.safe(), "{once:?}");
    let none = subject(KEEP, "void *p = q_alloc(); fill(&p);");
    assert!(none.unsafe_() && none.has("Leak"), "{none:?}");
    let twice = subject(KEEP, "void *p = q_alloc(); fill(&p); q_free(p); q_free(p);");
    assert!(twice.unsafe_() && twice.has("DoubleRelease"), "{twice:?}");
}

/// A slot the caller cannot place (uninitialised, possibly null, another
/// object's member) is never `Safe`: the kept arm's `Owned` position finds no
/// live identity.
#[test]
fn an_unplaced_slot_is_never_safe() {
    if !clang_available() {
        return;
    }
    for body in [
        "void *p; fill(&p); q_free(p);",
        "void *p = q_maybe_alloc(); fill(&p); q_free(p);",
    ] {
        let run = subject(KEEP, body);
        assert!(run.unknown() && run.has("UnknownAlias"), "{body}: {run:?}");
    }
    let member = g(&format!(
        "{KEEP} struct H {{ void *m; }}; \
         void subject(struct H *h) {{ void *p = h->m; fill(&p); q_free(p); }}"
    ));
    assert!(!member.safe() && member.has("UnknownAlias"), "{member:?}");
}

/// CHANGE PIN (text). A call whose slot actual is not `&NAME` is refused by
/// name, never read as either arm.
#[test]
fn an_other_actual_is_refused_by_name() {
    if !clang_available() {
        return;
    }
    let run = subject(KEEP, "void *p = 0; void **pp = &p; fill(pp); q_free(p);");
    assert!(run.unknown(), "{run:?}");
    assert!(
        run.refused(
            "call to `fill`, whose summary keeps or delivers its in-out slot, outside the \
             direct-call reading (an actual other than `&NAME`)"
        ),
        "{run:?}"
    );
}

/// CHANGE PIN. Every null-test spelling of the slot, an early exit that keeps
/// the slot, and a status assigned from the call.
#[test]
fn the_slot_tests_and_an_early_exit_read_alike() {
    if !clang_available() {
        return;
    }
    for callee in [
        "void fill(void **out) { if (*out == 0) *out = q_alloc(); }",
        "void fill(void **out) { if (*out != 0) return; *out = q_alloc(); }",
        "void fill(void **out) { if (*out) return; *out = q_alloc(); }",
    ] {
        let run = subject(callee, "void *p = 0; fill(&p); q_free(p);");
        assert!(run.safe(), "{callee}: {run:?}");
    }
    const EARLY: &str =
        "int fill(void **out, int c) { if (c) return -1; if (!*out) *out = q_alloc(); return 0; }";
    let nullable = g(&format!(
        "{EARLY} void subject(int c) {{ void *p = 0; int e; e = fill(&p, c); q_nullable_free(p); }}"
    ));
    assert!(nullable.safe(), "{nullable:?}");
    // The early exit leaves the null slot null: the delivery is nullable.
    let unchecked = g(&format!(
        "{EARLY} void subject(int c) {{ void *p = 0; int e; e = fill(&p, c); q_free(p); }}"
    ));
    assert!(!unchecked.safe(), "{unchecked:?}");
}

/// The callee's own row keeps rule 74l's refusal: the summary is what its
/// callers read, not a certificate of the body.
#[test]
fn the_callees_own_row_keeps_its_refusal() {
    if !clang_available() {
        return;
    }
    let run = g_function(KEEP, "fill");
    assert!(run.unknown(), "{run:?}");
    assert!(run.refused(&slot_refusal("q_alloc", "*out")), "{run:?}");
}

/// A store over a non-null slot is no keep: a partial-path store (rule 74l's
/// pin), a release of the kept object, a second store. No summary, so the
/// caller keeps the uncontracted call.
#[test]
fn an_overwrite_or_release_of_the_kept_object_publishes_nothing() {
    if !clang_available() {
        return;
    }
    for (callee, call) in [
        (
            "int fill(void **out, int c) { if (c) *out = q_alloc(); return c; }",
            "fill(&p, 1);",
        ),
        (
            "void fill(void **out) { void *old = *out; if (old) q_free(old); *out = q_alloc(); }",
            "fill(&p);",
        ),
        (
            "void fill(void **out) { if (!*out) *out = q_alloc(); *out = q_alloc(); }",
            "fill(&p);",
        ),
    ] {
        let run = subject(callee, &format!("void *p = 0; {call}"));
        assert!(run.unknown(), "{callee}: {run:?}");
        assert!(run.refused(&uncontracted("fill")), "{callee}: {run:?}");
    }
}

/// Shape 2 (`rd_kafka_assignment`, `rd_kafka_subscription`): the slot first
/// takes another object's member, then a fresh object when that is null and
/// `err` is 0; the early returns leave it unwritten. The member store
/// overwrites a kept object, so no keep-or-deliver summary; the return code
/// does not partition the slot. Refused (rule 85(3)).
#[test]
fn a_slot_first_holding_a_member_stays_refused() {
    if !clang_available() {
        return;
    }
    const ASSIGNMENT: &str = "struct H { void *m; }; \
        int fill(struct H *h, void **out, int err) { \
          if (!h) return -1; \
          *out = h->m; h->m = 0; \
          if (!*out && !err) { *out = q_alloc(); } \
          return err; }";
    let callee = g_function(ASSIGNMENT, "fill");
    assert!(callee.unknown(), "{callee:?}");
    let caller = g(&format!(
        "{ASSIGNMENT} void subject(struct H *h) {{ void *p = 0; int e; e = fill(h, &p, 0); }}"
    ));
    assert!(caller.unknown(), "{caller:?}");
    assert!(caller.refused(&uncontracted("fill")), "{caller:?}");
}

/// Shape 3, the sound part: a store partitioned by an explicit literal status
/// is the existing return-code reading (rule 11r(6)), unchanged here.
#[test]
fn a_literal_status_partition_keeps_its_reading() {
    if !clang_available() {
        return;
    }
    const PARTITIONED: &str =
        "int fill(void **out, int c) { if (c) return -1; *out = q_alloc(); return 0; }";
    let caller = g(&format!(
        "{PARTITIONED} void subject(int c) {{ void *p; if (fill(&p, c) == 0) q_free(p); }}"
    ));
    assert!(caller.safe(), "{caller:?}");
}

/// Shape 3, refused (`cio_file_native_lookup_user`/`_group`): a store, a
/// second store over it, and failure exits on which the slot holds a live
/// object or nothing. No reading of the failure status states the slot.
#[test]
fn an_unpartitioned_failure_store_stays_refused() {
    if !clang_available() {
        return;
    }
    const LOOKUP: &str = "int fill(int user, int fail, void **out) { \
        if (!user) { *out = q_alloc(); } \
        if (fail) return -1; \
        *out = q_alloc(); return 0; }";
    let callee = g_function(LOOKUP, "fill");
    assert!(callee.unknown(), "{callee:?}");
    let caller = g(&format!(
        "{LOOKUP} void subject(void) {{ void *p = 0; if (fill(0, 0, &p) == 0) q_free(p); }}"
    ));
    assert!(!caller.safe(), "{caller:?}");
}
