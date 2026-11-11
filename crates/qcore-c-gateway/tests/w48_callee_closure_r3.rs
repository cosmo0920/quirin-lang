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

//! W48-CALLEE-CLOSURE-R3 — controls for the two libc declarations this card
//! lands in `quirin-provider-libc` (`docs/reports/w48-callee-closure-r3/README.md`).
//!
//! Each module is loaded from its production provider file, byte for byte,
//! and exercised through the public gateway entry point on small held-out
//! bodies:
//!
//! * **accepting:** the shape of the tail-pause slice's `tail_file_lstat`,
//!   and a timer shape whose only call is `clock_gettime`, are `Safe`;
//! * **withdrawal:** without the card's sections each call is uncontracted;
//! * **refusing:** a borrowed position takes no custody, so an owned buffer
//!   handed to either call and dropped still leaks, and a released buffer is
//!   never an acceptable argument;
//! * **the header body:** librdkafka's `rd_clock` (Linux arm, `src/rdtime.h`)
//!   reads two members of the `struct timespec` whose address it handed to
//!   `clock_gettime`; with the declaration in force the call is modelled and
//!   the body was still refused, at its `return` (report section 3). Since
//!   W48-BOTTOM-UP-CALLEE-BODIES (gateway rule 12f) the body is `Safe`; the
//!   pin was changed deliberately.
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
const LIBC_SYS_STAT: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/sys_stat.qv");
const TIME_SECTION: &str = "// ## Card W48-CALLEE-CLOSURE-R3: `clock_gettime`";
const STAT_SECTION: &str = "// ## Card W48-CALLEE-CLOSURE-R3: `lstat`";

const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ size_t; \
typedef int clockid_t; \
typedef long long rd_ts_t; \
struct timespec { long tv_sec; long tv_nsec; }; \
struct stat { long st_size; long st_mode; }; \
void *malloc(size_t size); \
void free(void *ptr); \
int clock_gettime(clockid_t clock_id, struct timespec *tp); \
int lstat(const char *__restrict file, struct stat *__restrict buf);\n\
#define NULL ((void *)0)\n";

fn production() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        ("libc/time.qv".to_owned(), normalize_newlines(LIBC_TIME)),
        (
            "libc/sys_stat.qv".to_owned(),
            normalize_newlines(LIBC_SYS_STAT),
        ),
    ]))
    .unwrap_or_else(|error| panic!("production contract text rejected: {error}"))
}

fn before(text: &str, marker: &str) -> String {
    let text = normalize_newlines(text);
    let start = text
        .find(marker)
        .expect("the card's section is in the module");
    text[..start].to_owned()
}

/// The same modules without this card's two sections.
fn without_card() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        ("libc/time.qv".to_owned(), before(LIBC_TIME, TIME_SECTION)),
        (
            "libc/sys_stat.qv".to_owned(),
            before(LIBC_SYS_STAT, STAT_SECTION),
        ),
    ]))
    .unwrap_or_else(|error| panic!("contract text rejected: {error}"))
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent W48-CALLEE-CLOSURE-R3 test: {reason}");
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

fn summary_with(contracts: &ResourceContracts, body: &str, entry: &str) -> LegacyFunctionSummary {
    let source = format!("{DECLARATIONS}{body}\n");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        contracts,
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    program(&result)
        .functions
        .get(entry)
        .unwrap_or_else(|| panic!("missing function `{entry}`"))
        .clone()
}

fn summary(body: &str, entry: &str) -> LegacyFunctionSummary {
    summary_with(&production(), body, entry)
}

fn count(summary: &LegacyFunctionSummary, wanted: fn(&LegacyDiagnosticKind) -> bool) -> usize {
    summary
        .diagnostics
        .iter()
        .filter(|diagnostic| wanted(&diagnostic.kind))
        .count()
}

fn uncontracted(summary: &LegacyFunctionSummary, callee: &str) -> bool {
    let wanted = format!("call to uncontracted function `{callee}`");
    summary.diagnostics.iter().any(|d| {
        matches!(&d.kind, LegacyDiagnosticKind::Unsupported { description } if *description == wanted)
    })
}

/// `tail_file_lstat` (tail-pause slice, `plugins/in_tail/tail_file.c`, the
/// non-Windows arm) and a clock read whose value is kept as a scalar.
const ACCEPTING: [(&str, &str, &str); 2] = [
    (
        "tail_file_lstat",
        "lstat",
        "int a(void *ctx, const char *path, struct stat *st) { (void) ctx; return lstat(path, st); }",
    ),
    (
        "a clock read into caller storage",
        "clock_gettime",
        "int a(struct timespec *out) { return clock_gettime(1, out); }",
    ),
];

#[test]
fn every_accepting_shape_is_safe() {
    if !clang_available() {
        return;
    }
    for (name, _, body) in ACCEPTING {
        let summary = summary(body, "a");
        assert_eq!(
            summary.status,
            AnalysisStatus::Safe,
            "{name}: expected Safe, got {:?}",
            summary.diagnostics
        );
    }
}

#[test]
fn without_the_declarations_every_accepting_body_is_refused() {
    if !clang_available() {
        return;
    }
    let bare = without_card();
    for (name, callee, body) in ACCEPTING {
        let summary = summary_with(&bare, body, "a");
        assert_ne!(
            summary.status,
            AnalysisStatus::Safe,
            "{name}: must not be Safe without the card's declaration"
        );
        assert!(
            uncontracted(&summary, callee),
            "{name}: expected an uncontracted `{callee}`, got {:?}",
            summary.diagnostics
        );
    }
}

#[test]
fn a_borrowed_position_takes_no_custody() {
    if !clang_available() {
        return;
    }
    for (callee, call) in [
        ("lstat", "lstat(\"/x\", (struct stat *) p);"),
        ("clock_gettime", "clock_gettime(1, (struct timespec *) p);"),
    ] {
        // An owned buffer handed to the call and dropped still leaks.
        let leak = summary(
            &format!("void u(void) {{ char *p; p = malloc(64); if (!p) {{ return; }} {call} }}"),
            "u",
        );
        assert_ne!(
            leak.status,
            AnalysisStatus::Safe,
            "{callee}: {:?}",
            leak.diagnostics
        );
        assert!(
            count(&leak, |kind| matches!(
                kind,
                LegacyDiagnosticKind::Leak { .. }
            )) >= 1,
            "{callee}: expected a leak, got {:?}",
            leak.diagnostics
        );

        // A released buffer is never an acceptable argument.
        let released = summary(
            &format!(
                "void u(void) {{ char *p; p = malloc(64); if (!p) {{ return; }} free(p); {call} }}"
            ),
            "u",
        );
        assert_ne!(
            released.status,
            AnalysisStatus::Safe,
            "{callee}: {:?}",
            released.diagnostics
        );

        // Released after the call by its owner: Safe.
        let owned = summary(
            &format!(
                "void u(void) {{ char *p; p = malloc(64); if (!p) {{ return; }} {call} free(p); }}"
            ),
            "u",
        );
        assert_eq!(
            owned.status,
            AnalysisStatus::Safe,
            "{callee}: {:?}",
            owned.diagnostics
        );
    }
}

/// `rd_clock`'s Linux arm and `rd_kafka_timer_next`'s use of it
/// (`lib/librdkafka-2.15.0/src/rdtime.h:86-105`, `rdkafka_timer.c:252-271`,
/// the lock calls dropped). With the declaration in force the call is no
/// longer the body's refusal; what remains is pinned here.
#[test]
fn rd_clock_with_the_declaration_in_force() {
    if !clang_available() {
        return;
    }
    let body = "static rd_ts_t rd_clock(void) { struct timespec ts; clock_gettime(1, &ts); \
                return ((rd_ts_t)ts.tv_sec * 1000000LLU) + ((rd_ts_t)ts.tv_nsec / 1000LLU); }";
    let with = summary(body, "rd_clock");
    let bare = summary_with(&without_card(), body, "rd_clock");
    assert!(
        uncontracted(&bare, "clock_gettime"),
        "{:?}",
        bare.diagnostics
    );
    assert!(
        !uncontracted(&with, "clock_gettime"),
        "{:?}",
        with.diagnostics
    );
    // The call no longer refuses. This card pinned the `return` as the
    // remaining refusal: its operands read two scalar members of the local
    // aggregate `ts`, whose address the call took, and the typed access-path
    // reader refuses that place in a return (W48-RETURN-AGGREGATE-MEMBER-READ).
    // Changed deliberately by W48-BOTTOM-UP-CALLEE-BODIES (gateway rule 12f):
    // a refused scalar return value the statement position's W43 value
    // walker reads is lowered as that statement's value, so the body is Safe
    // (`tests/w48_bottom_up_callee_bodies.rs` holds the rule's controls).
    assert_eq!(with.status, AnalysisStatus::Safe, "{:?}", with.diagnostics);
    // The same member read in a statement is admitted, and a plain `return`
    // of the copied scalar is Safe. The refusal this card found was
    // positional (report section 3); since rule 12f the direct `return` of
    // the member is Safe as well.
    let copied = summary(
        "long p(void) { struct timespec ts; long s; clock_gettime(1, &ts); s = ts.tv_sec; return s; }",
        "p",
    );
    assert_eq!(
        copied.status,
        AnalysisStatus::Safe,
        "{:?}",
        copied.diagnostics
    );
    let returned = summary(
        "long p(void) { struct timespec ts; clock_gettime(1, &ts); return ts.tv_sec; }",
        "p",
    );
    assert_eq!(
        returned.status,
        AnalysisStatus::Safe,
        "{:?}",
        returned.diagnostics
    );
}
