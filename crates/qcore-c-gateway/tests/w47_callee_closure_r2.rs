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

//! W47-CALLEE-CLOSURE-R2 — controls for the two production declarations this
//! card lands (`docs/reports/w47-callee-closure-r2/README.md`).
//!
//! Each declaration is loaded from its production provider module, byte for
//! byte (the librdkafka section from its marker, because the module's libc
//! includes are not resolved by this standalone loader), and exercised through
//! the public gateway entry points on small held-out bodies:
//!
//! * **accepting:** the shapes of the functions the card moves are `Safe`;
//! * **withdrawal:** without the card's sections each call is uncontracted;
//! * **refusing, arena:** the result of `cfl_arena_calloc` is interior to its
//!   arena, so a use of it after the arena's storage is released is a
//!   use-after-release candidate, handing it to `free` is never `Safe`, and a
//!   released arena is refused as an argument;
//! * **refusing, owned result:** `rd_file_read`'s result is owned and
//!   nullable: dropping it on the non-null path leaks, releasing it twice is a
//!   violation candidate, and both null-handling forms are `Safe`.
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
const CFL_SDS: &str =
    include_str!("../../../providers/quirin-provider-cfl/contracts/cfl/sds_lifetime.qv");
/// `librdkafka/memory.qv` includes libc parents, which this standalone loader
/// does not resolve; the card's declaration is its final section.
const RDKAFKA_MEMORY: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv");
const CFL_SECTION: &str = "// ## Card W47-CALLEE-CLOSURE-R2: `cfl_arena_calloc`";
const RDKAFKA_SECTION: &str = "// ## Card W47-CALLEE-CLOSURE-R2: `rd_file_read`";

const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ size_t; \
struct cfl_arena; \
typedef struct kv kv; \
typedef struct quantile quantile; \
void *malloc(size_t size); \
void free(void *ptr); \
void *cfl_arena_calloc(struct cfl_arena *arena, size_t count, size_t size); \
char *rd_file_read(const char *file_path, size_t *size, size_t max_size);\n\
#define NULL ((void *)0)\n";

fn section(text: &str, marker: &str) -> String {
    let text = normalize_newlines(text);
    let start = text
        .find(marker)
        .expect("the card's section is in the module");
    format!("module contract;\n{}", &text[start..])
}

fn production() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        (
            "cfl/sds_lifetime.qv".to_owned(),
            normalize_newlines(CFL_SDS),
        ),
        (
            "librdkafka/w47_file_read.qv".to_owned(),
            section(RDKAFKA_MEMORY, RDKAFKA_SECTION),
        ),
    ]))
    .unwrap_or_else(|error| panic!("production contract text rejected: {error}"))
}

/// The same modules without this card's declarations.
fn without_card() -> ResourceContracts {
    let text = normalize_newlines(CFL_SDS);
    let start = text.find(CFL_SECTION).expect("card section present");
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        ("cfl/sds_lifetime.qv".to_owned(), text[..start].to_owned()),
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
            eprintln!("skipping Clang-dependent W47-CALLEE-CLOSURE-R2 test: {reason}");
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

/// The shapes of the three functions the card moves, each on the callee it
/// declares: the two cmetrics list initialisers and the librdkafka file reader.
const ACCEPTING: [(&str, &str); 4] = [
    (
        "initialize_attribute_list",
        "kv **a(struct cfl_arena *arena, size_t element_count) { kv **attribute_list; attribute_list = cfl_arena_calloc(arena, element_count + 1, sizeof(kv *)); return attribute_list; }",
    ),
    (
        "initialize_summary_value_at_quantile_list",
        "quantile **a(struct cfl_arena *arena, size_t element_count) { quantile **list; list = cfl_arena_calloc(arena, element_count + 1, sizeof(quantile *)); return list; }",
    ),
    (
        "rd_kafka_oidc_assertion_read_from_file",
        "char *a(const char *file_path) { if (!file_path) return NULL; const size_t max_size = 1024 * 1024; return rd_file_read(file_path, NULL, max_size); }",
    ),
    (
        "rd_file_read released on its non-null path",
        "void a(const char *path) { char *b; size_t n; b = rd_file_read(path, &n, 0); if (b) { free(b); } }",
    ),
];

#[test]
fn every_moved_shape_is_accepted() {
    if !clang_available() {
        return;
    }
    for (name, body) in ACCEPTING {
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
    for (name, body) in ACCEPTING {
        let summary = summary_with(&bare, body, "a");
        assert_ne!(
            summary.status,
            AnalysisStatus::Safe,
            "{name}: must not be Safe without the card's declaration"
        );
        assert!(
            summary.diagnostics.iter().any(|d| matches!(
                &d.kind,
                LegacyDiagnosticKind::Unsupported { description }
                    if description.starts_with("call to uncontracted function")
            )),
            "{name}: expected an uncontracted call, got {:?}",
            summary.diagnostics
        );
    }
}

#[test]
fn an_arena_allocation_dies_with_its_arena() {
    if !clang_available() {
        return;
    }
    // `malloc` stands in for a live arena the body owns, so its release is
    // visible: a read of the interior result after it is a candidate.
    let after = summary(
        "int u(void) { struct cfl_arena *m; char *p; m = malloc(64); if (!m) { return 0; } p = cfl_arena_calloc(m, 1, 8); free(m); if (p) { return p[0]; } return 1; }",
        "u",
    );
    assert_eq!(
        after.status,
        AnalysisStatus::Unsafe,
        "{:?}",
        after.diagnostics
    );
    assert!(
        count(&after, |kind| matches!(
            kind,
            LegacyDiagnosticKind::UseAfterRelease { .. }
        )) >= 1,
        "{:?}",
        after.diagnostics
    );

    // An already released arena is never an acceptable argument.
    let released = summary(
        "void *u(void) { struct cfl_arena *m; char *p; m = malloc(64); if (!m) { return NULL; } free(m); p = cfl_arena_calloc(m, 1, 8); return p; }",
        "u",
    );
    assert_ne!(
        released.status,
        AnalysisStatus::Safe,
        "{:?}",
        released.diagnostics
    );

    // The result is not the caller's to free: never Safe.
    let freed = summary(
        "void u(struct cfl_arena *arena) { char *p; p = cfl_arena_calloc(arena, 1, 8); free(p); }",
        "u",
    );
    assert_ne!(
        freed.status,
        AnalysisStatus::Safe,
        "{:?}",
        freed.diagnostics
    );

    // Borrowing an owned arena releases nothing: the arena is still released
    // by its owner, here `free`, and without it the stand-in leaks.
    let leak = summary(
        "void u(void) { struct cfl_arena *m; m = malloc(64); if (!m) { return; } cfl_arena_calloc(m, 1, 8); }",
        "u",
    );
    assert_eq!(
        leak.status,
        AnalysisStatus::Unsafe,
        "{:?}",
        leak.diagnostics
    );
    assert_eq!(
        count(&leak, |kind| matches!(
            kind,
            LegacyDiagnosticKind::Leak { .. }
        )),
        1,
        "{:?}",
        leak.diagnostics
    );
}

#[test]
fn a_file_buffer_is_owned_and_nullable() {
    if !clang_available() {
        return;
    }
    // Dropped on the non-null path: a leak candidate.
    let dropped = summary(
        "void l(const char *p) { char *b; b = rd_file_read(p, NULL, 0); }",
        "l",
    );
    assert_eq!(
        dropped.status,
        AnalysisStatus::Unsafe,
        "{:?}",
        dropped.diagnostics
    );
    assert_eq!(
        count(&dropped, |kind| matches!(
            kind,
            LegacyDiagnosticKind::Leak { .. }
        )),
        1,
        "{:?}",
        dropped.diagnostics
    );

    // Released twice: never Safe.
    let twice = summary(
        "void l(const char *p) { char *b; b = rd_file_read(p, NULL, 0); if (!b) { return; } free(b); free(b); }",
        "l",
    );
    assert_ne!(
        twice.status,
        AnalysisStatus::Safe,
        "{:?}",
        twice.diagnostics
    );

    // The null path needs no release: an early return on NULL is Safe.
    let early = summary(
        "void l(const char *p) { char *b; b = rd_file_read(p, NULL, 0); if (!b) { return; } free(b); }",
        "l",
    );
    assert_eq!(
        early.status,
        AnalysisStatus::Safe,
        "{:?}",
        early.diagnostics
    );
}
