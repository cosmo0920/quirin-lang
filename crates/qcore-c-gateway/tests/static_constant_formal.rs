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

//! Card FB-CHUNK-LIFECYCLE (gateway rule 90): a `static` function whose
//! integer formal every call in the unit fixes to a constant, the shape of
//! Fluent Bit's `sb_remove_chunk_from_segregated_backlog(target_chunk,
//! backlog, destroy)` (`plugins/in_storage_backlog/sb.c:214-246`, its only
//! call passing `FLB_TRUE`). The base verdict is in each comment.
//! Conventions as in `clang_gap_phase1.rs`: `Safe` here is over the synthetic
//! `q_*` fixture assumptions and promotes nothing; an `Unsafe` is a
//! provisional verdict over the same fixtures.

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::*;

/// The probed body: an owned object released only on the `destroy` arm.
const BODY: &str = "static void subject(int destroy) \
     { void *p = q_alloc(); if (destroy) q_free(p); }";

/// The same release on the negated test.
const NEGATED: &str = "static void subject(int keep) \
     { void *p = q_alloc(); if (!keep) q_free(p); }";

const PREMISE: &str = "static-call-sites-closed";

fn with_callers(body: &str, callers: &str) -> G {
    g(&format!("{body}\n{callers}"))
}

#[test]
fn every_call_fixing_the_formal_selects_its_arm() {
    if !clang_available() {
        return;
    }
    // Base: Unsafe (the false arm's leak). Tip: Safe, every call passing a
    // nonzero constant.
    for callers in [
        "void caller(void) { subject(1); }",
        "void caller(void) { subject(1); subject(1); }",
        "void caller(void) { subject(1); } void other(void) { subject((1)); }",
        "void caller(void) { subject(1); subject(2); }",
        "static void subject(int destroy);\nvoid caller(void) { subject(1); }",
        "void caller(int x) { if (x) subject(1); else subject(1); }",
        "void caller(void) { (void) subject(1); }",
    ] {
        let row = with_callers(BODY, callers);
        assert!(row.safe(), "{callers}: {row:?}");
    }
    // Base: Unsafe. Tip: Safe, the negated test under a zero actual.
    let negated = with_callers(NEGATED, "void caller(void) { subject(0); }");
    assert!(negated.safe(), "{negated:?}");
    // Base and tip: Unsafe. Under these actuals the selected arm is the one
    // that keeps the object, and every invocation takes it.
    for (body, callers) in [
        (BODY, "void caller(void) { subject(0); }"),
        (NEGATED, "void caller(void) { subject(1); }"),
    ] {
        let row = with_callers(body, callers);
        assert!(row.unsafe_(), "{callers}: {row:?}");
    }
}

#[test]
fn the_premise_is_carried_by_the_row() {
    if !clang_available() {
        return;
    }
    let text = format!("{DECLARATIONS}\n{BODY}\nvoid caller(void) {{ subject(1); }}\n");
    let outcome = qcore_c_gateway::legacy_c::analyze_unrestricted_c11(
        &qcore_c_gateway::legacy_c::C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .expect("the fixture parses");
    let rendered = format!("{outcome:?}");
    assert!(rendered.contains(PREMISE), "{rendered}");
    // No selection, no premise.
    let text = format!("{DECLARATIONS}\n{BODY}\nvoid caller(int x) {{ subject(x); }}\n");
    let outcome = qcore_c_gateway::legacy_c::analyze_unrestricted_c11(
        &qcore_c_gateway::legacy_c::C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .expect("the fixture parses");
    assert!(!format!("{outcome:?}").contains(PREMISE));
}

#[test]
fn anything_short_of_every_condition_keeps_the_old_reading() {
    if !clang_available() {
        return;
    }
    // Base and tip: Unsafe (or Unknown where the body has its own refusal),
    // never Safe: the false arm is kept.
    for (body, callers) in [
        (BODY, "void caller(void) { subject(1); subject(0); }"),
        (BODY, "void caller(int x) { subject(x); }"),
        (
            BODY,
            "void caller(void) { void (*fp)(int) = subject; fp(1); }",
        ),
        (
            BODY,
            "void caller(void) { void (*fp)(int) = &subject; fp(1); }",
        ),
        (
            BODY,
            "void (*table[1])(int) = { subject };\nvoid caller(void) { subject(1); }",
        ),
        (BODY, ""),
        (BODY, "void caller(void) { subject(10000); }"),
        (BODY, "void caller(void) { subject(01); }"),
        (BODY, "void caller(void) { subject(-1); }"),
        (BODY, "void caller(void) { subject(1u); }"),
        (BODY, "void caller(void) { subject((int) 1); }"),
        (BODY, "void caller(void) { void subject(int); subject(1); }"),
        (
            "void subject(int destroy) { void *p = q_alloc(); if (destroy) q_free(p); }",
            "void caller(void) { subject(1); }",
        ),
        (
            "static void subject(volatile int destroy) \
             { void *p = q_alloc(); if (destroy) q_free(p); }",
            "void caller(void) { subject(1); }",
        ),
        (
            "static void subject(char destroy) { void *p = q_alloc(); if (destroy) q_free(p); }",
            "void caller(void) { subject(1); }",
        ),
        (
            "static void subject(int destroy) \
             { void *p = q_alloc(); if (destroy && opaque()) q_free(p); }",
            "void caller(void) { subject(1); }",
        ),
        (
            "static void subject(int destroy) \
             { void *p = q_alloc(); destroy = 0; if (destroy) q_free(p); }",
            "void caller(void) { subject(1); }",
        ),
        (
            "static void subject(int destroy) \
             { void *p = q_alloc(); { int destroy = 0; if (destroy) q_free(p); } }",
            "void caller(void) { subject(1); }",
        ),
        (
            "static void subject(int destroy) \
             { void *p = q_alloc(); while (destroy) { q_free(p); return; } }",
            "void caller(void) { subject(1); }",
        ),
    ] {
        let row = with_callers(body, callers);
        assert!(!row.safe(), "{body} {callers}: {row:?}");
    }
}
