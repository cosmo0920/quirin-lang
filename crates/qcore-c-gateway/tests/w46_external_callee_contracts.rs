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

//! W46-EXTERNAL-CALLEE-CONTRACTS — controls for the production declarations
//! this card lands (`docs/reports/w46-external-callee-contracts/README.md`).
//!
//! Every declaration is loaded from its production provider module, byte for
//! byte, and exercised through the public gateway entry points on small
//! held-out bodies:
//!
//! * **accepting:** correct use of each callee is `Safe`;
//! * **refusing:** a misuse is a leak candidate, a violation candidate or a
//!   refusal (`Unknown`), never `Safe` — a borrowed position releases nothing,
//!   so an owned actual handed only to one of these callees still leaks; a
//!   released heap actual is refused by the borrowed-liveness gate; a released
//!   descriptor handed to `fcntl` is a use-after-release candidate; and `fcntl`
//!   at any command but `F_SETFD` (`2`) is outside the declaration's premise;
//! * **withdrawal:** without the declaration each call is uncontracted again.
//!
//! No declaration here has an owned or nullable result (each result is
//! `resource_return_none()`), so there is no nullable-result control to write:
//! the one nullable effect in these fixtures is `malloc`'s, which is the libc
//! module's own and already controlled by its card.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/text.rs"]
mod text;
use text::normalize_newlines;

const UNISTD: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/unistd.qv");
const LIBC_MEMORY: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv");
const MPACK_WRITER: &str =
    include_str!("../../../providers/quirin-provider-mpack/contracts/mpack/writer.qv");
const CFL_SDS: &str =
    include_str!("../../../providers/quirin-provider-cfl/contracts/cfl/sds_lifetime.qv");
const MONKEY_LOGGING: &str =
    include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/logging.qv");
/// `librdkafka/memory.qv` includes libc parents, which this standalone
/// loader does not resolve; the card's declaration is its final section, which
/// is taken byte for byte from the section marker to the end of the module.
const RDKAFKA_MEMORY: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv");
const RDKAFKA_SECTION: &str = "// ## Card W46-EXTERNAL-CALLEE-CONTRACTS: `rd_atomic32_get`";

const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ size_t; \
typedef unsigned int uint32_t; \
typedef int int32_t; \
typedef char *cfl_sds_t; \
typedef struct mpack_writer_t mpack_writer_t; \
typedef struct { volatile int32_t val; } rd_atomic32_t; \
void *malloc(size_t size); \
void free(void *ptr); \
int open(const char *file, int oflag, ...); \
int close(int fd); \
int fcntl(int fd, int cmd, ...); \
void mpack_finish_map(mpack_writer_t *writer); \
void mpack_finish_array(mpack_writer_t *writer); \
void mpack_write_true(mpack_writer_t *writer); \
void mpack_write_false(mpack_writer_t *writer); \
void mpack_write_bin(mpack_writer_t *writer, const char *data, uint32_t count); \
size_t cfl_sds_len(cfl_sds_t s); \
void mk_utils_libc_error(char *caller, char *file, int line); \
int32_t rd_atomic32_get(rd_atomic32_t *ra);\n\
#define F_GETFL 3\n\
#define F_SETFD 2\n\
#define F_DUPFD 0\n\
#define FD_CLOEXEC 1\n";

const OUTSIDE_PREMISE: &str = "call to `fcntl` outside its declared literal-argument premise";

fn rdkafka_section() -> String {
    let text = normalize_newlines(RDKAFKA_MEMORY);
    let start = text
        .find(RDKAFKA_SECTION)
        .expect("the card's section is in librdkafka/memory.qv");
    format!("module contract;\n{}", &text[start..])
}

fn production() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/unistd.qv".to_owned(), normalize_newlines(UNISTD)),
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        (
            "mpack/writer.qv".to_owned(),
            normalize_newlines(MPACK_WRITER),
        ),
        (
            "cfl/sds_lifetime.qv".to_owned(),
            normalize_newlines(CFL_SDS),
        ),
        (
            "monkey/logging.qv".to_owned(),
            normalize_newlines(MONKEY_LOGGING),
        ),
        ("librdkafka/w46_atomic.qcore".to_owned(), rdkafka_section()),
    ]))
    .unwrap_or_else(|error| panic!("production contract text rejected: {error}"))
}

/// The same modules without this card's declarations: libc's allocator,
/// `open`/`close` and the earlier writer/sds/logging declarations stay.
fn without_card() -> ResourceContracts {
    let cut = |text: &str, marker: &str| -> String {
        let text = normalize_newlines(text);
        let start = text.find(marker).expect("card section present");
        text[..start].to_owned()
    };
    load_resource_contract_sources(&BTreeMap::from([
        (
            "libc/unistd.qv".to_owned(),
            cut(UNISTD, "// ## Card W46-EXTERNAL-CALLEE-CONTRACTS"),
        ),
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        (
            "mpack/writer.qv".to_owned(),
            cut(MPACK_WRITER, "// ## Card W46-EXTERNAL-CALLEE-CONTRACTS"),
        ),
        (
            "cfl/sds_lifetime.qv".to_owned(),
            cut(CFL_SDS, "// ## Card W46-EXTERNAL-CALLEE-CONTRACTS"),
        ),
        (
            "monkey/logging.qv".to_owned(),
            cut(MONKEY_LOGGING, "// ## Card W46-EXTERNAL-CALLEE-CONTRACTS"),
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
            eprintln!("skipping Clang-dependent W46-EXTERNAL-CALLEE-CONTRACTS test: {reason}");
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

fn unsupported(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect()
}

fn leaks(summary: &LegacyFunctionSummary) -> usize {
    summary
        .diagnostics
        .iter()
        .filter(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. }))
        .count()
}

/// The descriptor kind's use-after-release candidate (contract spec C04.6
/// rule 6), published in the `use_after_release` category.
fn use_after_close(summary: &LegacyFunctionSummary) -> usize {
    summary
        .diagnostics
        .iter()
        .filter(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::UseAfterClose { .. }))
        .count()
}

fn assert_safe(summary: &LegacyFunctionSummary) {
    assert_eq!(
        summary.status,
        AnalysisStatus::Safe,
        "expected Safe: {:?}",
        summary.diagnostics
    );
}

/// One accepting body per contracted callee: each calls it the way the
/// corpus's moved functions do, on a formal it borrows.
const ACCEPTING: [(&str, &str); 7] = [
    (
        "mpack_finish",
        "void a(mpack_writer_t *w) { mpack_finish_map(w); mpack_finish_array(w); mpack_finish_map(w); }",
    ),
    (
        "mpack_bool",
        "void a(mpack_writer_t *w, int b) { if (b) { mpack_write_true(w); } else { mpack_write_false(w); } }",
    ),
    (
        "mpack_bin",
        "void a(mpack_writer_t *w, cfl_sds_t s) { size_t n; n = cfl_sds_len(s); mpack_write_bin(w, s, n); }",
    ),
    (
        "cfl_sds_len",
        "size_t a(cfl_sds_t s) { return cfl_sds_len(s); }",
    ),
    (
        "mk_utils_libc_error",
        "void a(void) { mk_utils_libc_error(\"write\", \"mk_scheduler.c\", 878); }",
    ),
    (
        "rd_atomic32_get",
        "int a(rd_atomic32_t *counter) { return rd_atomic32_get(counter); }",
    ),
    (
        "fcntl",
        "int a(const char *path) { int fd; fd = open(path, 0); if (fd == -1) { return -1; } fcntl(fd, F_SETFD, FD_CLOEXEC); return fd; }",
    ),
];

#[test]
fn every_contracted_callee_accepts_its_correct_use() {
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
            unsupported(&summary)
                .iter()
                .any(|d| d.starts_with("call to uncontracted function")),
            "{name}: expected an uncontracted call, got {:?}",
            summary.diagnostics
        );
    }
}

#[test]
fn a_borrowed_position_releases_nothing_so_an_owned_actual_still_leaks() {
    if !clang_available() {
        return;
    }
    let bodies = [
        "void l(void) { mpack_writer_t *w; w = malloc(64); if (!w) { return; } mpack_finish_map(w); }",
        "void l(void) { mpack_writer_t *w; w = malloc(64); if (!w) { return; } mpack_write_true(w); }",
        "void l(mpack_writer_t *w) { char *d; d = malloc(4); if (!d) { return; } mpack_write_bin(w, d, 4); }",
        "void l(void) { cfl_sds_t s; s = malloc(16); if (!s) { return; } cfl_sds_len(s); }",
        "void l(void) { char *c; c = malloc(8); if (!c) { return; } mk_utils_libc_error(c, \"f.c\", 1); }",
        "void l(void) { rd_atomic32_t *r; r = malloc(sizeof(rd_atomic32_t)); if (!r) { return; } rd_atomic32_get(r); }",
    ];
    for body in bodies {
        let summary = summary(body, "l");
        assert_eq!(
            summary.status,
            AnalysisStatus::Unsafe,
            "{body}: {:?}",
            summary.diagnostics
        );
        assert_eq!(leaks(&summary), 1, "{body}: {:?}", summary.diagnostics);
    }
}

#[test]
fn a_released_heap_actual_is_refused_not_accepted() {
    if !clang_available() {
        return;
    }
    let bodies = [
        "void u(void) { mpack_writer_t *w; w = malloc(64); if (!w) { return; } free(w); mpack_write_false(w); }",
        "void u(void) { cfl_sds_t s; s = malloc(16); if (!s) { return; } free(s); cfl_sds_len(s); }",
    ];
    for body in bodies {
        let summary = summary(body, "u");
        assert_ne!(
            summary.status,
            AnalysisStatus::Safe,
            "{body}: {:?}",
            summary.diagnostics
        );
    }
}

#[test]
fn fcntl_is_declared_only_at_f_setfd() {
    if !clang_available() {
        return;
    }
    // `F_GETFL` (3) and `F_DUPFD` (0, which creates a descriptor) are outside
    // the premise, so the callee is undeclared there.
    for command in ["F_GETFL", "F_DUPFD", "4"] {
        let body = format!(
            "int p(const char *path) {{ int fd; int r; fd = open(path, 0); if (fd == -1) {{ return -1; }} r = fcntl(fd, {command}); close(fd); return r; }}"
        );
        let summary = summary(&body, "p");
        assert_eq!(
            summary.status,
            AnalysisStatus::Unknown,
            "{command}: {:?}",
            summary.diagnostics
        );
        assert!(
            unsupported(&summary).iter().any(|d| d == OUTSIDE_PREMISE),
            "{command}: {:?}",
            summary.diagnostics
        );
    }
    // A command that only reaches `2` through arithmetic is not the literal.
    let summary = summary("void p(int fd) { fcntl(fd, 1 + 1, FD_CLOEXEC); }", "p");
    assert_ne!(
        summary.status,
        AnalysisStatus::Safe,
        "{:?}",
        summary.diagnostics
    );
}

#[test]
fn fcntl_at_f_setfd_borrows_the_descriptor() {
    if !clang_available() {
        return;
    }
    // Setting the flag releases nothing: the descriptor still leaks.
    let leak = summary(
        "void k(const char *path) { int fd; fd = open(path, 0); if (fd == -1) { return; } fcntl(fd, F_SETFD, FD_CLOEXEC); }",
        "k",
    );
    assert_eq!(
        leak.status,
        AnalysisStatus::Unsafe,
        "{:?}",
        leak.diagnostics
    );
    assert_eq!(leaks(&leak), 1, "{:?}", leak.diagnostics);

    // A use after `close` is a use-after-release candidate.
    let after = summary(
        "void k(const char *path) { int fd; fd = open(path, 0); if (fd == -1) { return; } close(fd); fcntl(fd, F_SETFD, FD_CLOEXEC); }",
        "k",
    );
    assert_eq!(
        after.status,
        AnalysisStatus::Unsafe,
        "{:?}",
        after.diagnostics
    );
    assert_eq!(use_after_close(&after), 1, "{:?}", after.diagnostics);

    // Opened, flagged and closed: Safe.
    assert_safe(&summary(
        "void k(const char *path) { int fd; fd = open(path, 0); if (fd == -1) { return; } fcntl(fd, F_SETFD, FD_CLOEXEC); close(fd); }",
        "k",
    ));
}
