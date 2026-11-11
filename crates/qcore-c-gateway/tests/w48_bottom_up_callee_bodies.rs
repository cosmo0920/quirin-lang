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

//! W48-BOTTOM-UP-CALLEE-BODIES (gateway rule 12f) — controls for a scalar
//! `return` value read as the value of a statement
//! (`docs/reports/w48-bottom-up-callee-bodies/README.md`).
//!
//! The libc modules are loaded from their production provider files, byte
//! for byte, and every fixture goes through the public gateway entry point:
//!
//! * **accepting:** a scalar member of an address-taken automatic aggregate
//!   returned directly (`rd_clock`'s shape), a typed access through a live
//!   pointer, and the statement form of each, are `Safe` alike;
//! * **the summary is carried:** a caller of an `rd_clock`-shaped `static`
//!   helper is `Safe` through the helper's validated summary (the bottom-up
//!   move the card measures on `rd_kafka_timer_next`);
//! * **refusing:** a read through a released base is reported, an owned
//!   object dropped before a scalar return still leaks, and a pointer result,
//!   a pointer converted to an integer, a value holding an uncontracted call
//!   or a pointer member stay refused.
//!
//! The candidates in these fixtures are fixture findings, not claims about
//! any upstream body.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/text.rs"]
mod text;
use text::normalize_newlines;

const LIBC_MEMORY: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv");
const LIBC_TIME: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/time.qv");

const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ size_t; \
typedef int clockid_t; \
typedef long long rd_ts_t; \
struct timespec { long tv_sec; long tv_nsec; }; \
struct holder { char *p; long n; }; \
void *malloc(size_t size); \
void free(void *ptr); \
int clock_gettime(clockid_t clock_id, struct timespec *tp); \
void fill(struct holder *h);\n\
#define NULL ((void *)0)\n";

const RETURN_REFUSAL: &str = "unmodeled call, aggregate or indirect access in return expression";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        ("libc/time.qv".to_owned(), normalize_newlines(LIBC_TIME)),
    ]))
    .unwrap_or_else(|error| panic!("production contract text rejected: {error}"))
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent W48-BOTTOM-UP-CALLEE-BODIES test: {reason}");
            false
        }
    }
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        }
        | UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        other => panic!("expected an analyzable summary: {other:?}"),
    }
}

fn summary(body: &str, entry: &str) -> LegacyFunctionSummary {
    let source = format!("{DECLARATIONS}{body}\n");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    program(&result)
        .functions
        .get(entry)
        .unwrap_or_else(|| panic!("missing function `{entry}`"))
        .clone()
}

fn return_refused(summary: &LegacyFunctionSummary) -> bool {
    summary.diagnostics.iter().any(|d| {
        matches!(&d.kind, LegacyDiagnosticKind::Unsupported { description }
            if description == RETURN_REFUSAL)
    })
}

fn has(summary: &LegacyFunctionSummary, wanted: fn(&LegacyDiagnosticKind) -> bool) -> bool {
    summary
        .diagnostics
        .iter()
        .any(|diagnostic| wanted(&diagnostic.kind))
}

/// Each accepting return and its statement form (`T tmp = E; return tmp;`),
/// which the statement position already decided.
const ACCEPTING: [(&str, &str, &str); 4] = [
    (
        "rd_clock (rdtime.h, Linux arm)",
        "rd_ts_t a(void) { struct timespec ts; clock_gettime(1, &ts); \
         return ((rd_ts_t)ts.tv_sec * 1000000LLU) + ((rd_ts_t)ts.tv_nsec / 1000LLU); }",
        "rd_ts_t a(void) { struct timespec ts; rd_ts_t tmp; clock_gettime(1, &ts); \
         tmp = ((rd_ts_t)ts.tv_sec * 1000000LLU) + ((rd_ts_t)ts.tv_nsec / 1000LLU); return tmp; }",
    ),
    (
        "one member of an address-taken aggregate",
        "long a(void) { struct timespec ts; clock_gettime(1, &ts); return ts.tv_sec; }",
        "long a(void) { struct timespec ts; long tmp; clock_gettime(1, &ts); tmp = ts.tv_sec; return tmp; }",
    ),
    (
        "a member read through a live owned pointer, released after the read",
        "long a(void) { struct timespec *t = malloc(sizeof *t); long v; if (!t) return 0; \
         clock_gettime(1, t); v = t->tv_sec; free(t); return v + 0; }",
        "long a(void) { struct timespec *t = malloc(sizeof *t); long v; long tmp; if (!t) return 0; \
         clock_gettime(1, t); v = t->tv_sec; free(t); tmp = v + 0; return tmp; }",
    ),
    (
        "two members of an aggregate an uncontracted call never saw",
        "long a(void) { struct timespec ts; ts.tv_sec = 1; ts.tv_nsec = 2; \
         clock_gettime(1, &ts); return ts.tv_sec - ts.tv_nsec; }",
        "long a(void) { struct timespec ts; long tmp; ts.tv_sec = 1; ts.tv_nsec = 2; \
         clock_gettime(1, &ts); tmp = ts.tv_sec - ts.tv_nsec; return tmp; }",
    ),
];

#[test]
fn every_accepting_return_is_safe_as_its_statement_form_is() {
    if !clang_available() {
        return;
    }
    for (name, returned, statement) in ACCEPTING {
        let returned = summary(returned, "a");
        let statement = summary(statement, "a");
        assert_eq!(
            statement.status,
            AnalysisStatus::Safe,
            "{name} (statement form): {:?}",
            statement.diagnostics
        );
        assert_eq!(
            returned.status,
            AnalysisStatus::Safe,
            "{name}: {:?}",
            returned.diagnostics
        );
    }
}

/// `rd_kafka_timer_next`'s use of `rd_clock` (`rdkafka_timer.c:252-271`,
/// the lock calls dropped): the helper's own body validates, so its
/// summary serves the caller in the same unit.
#[test]
fn the_validated_helper_summary_serves_its_caller() {
    if !clang_available() {
        return;
    }
    let body = "static rd_ts_t rd_clock(void) { struct timespec ts; clock_gettime(1, &ts); \
                return ((rd_ts_t)ts.tv_sec * 1000000LLU) + ((rd_ts_t)ts.tv_nsec / 1000LLU); } \
                rd_ts_t next(rd_ts_t at) { rd_ts_t delta; rd_ts_t now = rd_clock(); \
                if (at <= now) delta = 0; else delta = at - now; return delta; }";
    let caller = summary(body, "next");
    assert_eq!(
        caller.status,
        AnalysisStatus::Safe,
        "{:?}",
        caller.diagnostics
    );
}

#[test]
fn a_read_through_a_released_base_is_reported() {
    if !clang_available() {
        return;
    }
    let released = summary(
        "long a(void) { struct timespec *t = malloc(sizeof *t); if (!t) return 0; \
         t->tv_sec = 1; free(t); return t->tv_sec + 1; }",
        "a",
    );
    assert_ne!(
        released.status,
        AnalysisStatus::Safe,
        "{:?}",
        released.diagnostics
    );
    assert!(
        has(&released, |kind| matches!(
            kind,
            LegacyDiagnosticKind::UseAfterRelease { .. }
        )),
        "expected a use after release, got {:?}",
        released.diagnostics
    );
}

#[test]
fn a_scalar_return_escapes_nothing() {
    if !clang_available() {
        return;
    }
    // The owned object is dropped at the return: the scalar carries none of
    // its custody, so the leak stays.
    let leaked = summary(
        "long a(void) { struct timespec *t = malloc(sizeof *t); if (!t) return 0; \
         t->tv_sec = 1; return t->tv_sec + 1; }",
        "a",
    );
    assert_ne!(
        leaked.status,
        AnalysisStatus::Safe,
        "{:?}",
        leaked.diagnostics
    );
    assert!(
        has(&leaked, |kind| matches!(
            kind,
            LegacyDiagnosticKind::Leak { .. }
        )),
        "expected a leak, got {:?}",
        leaked.diagnostics
    );
}

#[test]
fn every_value_outside_the_walker_keeps_its_refusal() {
    if !clang_available() {
        return;
    }
    // The control: behind the same uncontracted call, a scalar member's
    // return is read (its refusal is gone; the call keeps the body Unknown).
    let scalar = summary(
        "long a(void) { struct holder h; fill(&h); return h.n; }",
        "a",
    );
    assert_ne!(
        scalar.status,
        AnalysisStatus::Safe,
        "{:?}",
        scalar.diagnostics
    );
    assert!(!return_refused(&scalar), "{:?}", scalar.diagnostics);
    for (name, body) in [
        (
            "a pointer result",
            "char *a(void) { struct holder h; fill(&h); return h.p; }",
        ),
        (
            "a pointer member converted to an integer",
            "long a(void) { struct holder h; fill(&h); return (long) h.p; }",
        ),
        (
            "a value holding a call",
            "long labs_like(long v); \
             long a(void) { struct holder h; fill(&h); return h.n + labs_like(h.n); }",
        ),
    ] {
        let refused = summary(body, "a");
        assert_ne!(
            refused.status,
            AnalysisStatus::Safe,
            "{name}: {:?}",
            refused.diagnostics
        );
        assert!(
            return_refused(&refused),
            "{name}: expected the return refusal, got {:?}",
            refused.diagnostics
        );
    }
}
