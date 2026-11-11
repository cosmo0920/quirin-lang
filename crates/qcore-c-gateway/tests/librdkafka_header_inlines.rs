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

//! LIBRDKAFKA-HEADER-INLINES: controls for the production declarations of
//! `librdkafka/header_inlines.qv`
//! (`docs/reports/librdkafka-header-inlines/README.md`).
//!
//! The module is loaded byte for byte, beside libc's `malloc`/`free`, and
//! exercised through the public gateway entry point on small held-out bodies:
//!
//! * **accepting:** each declaration family, used as the corpus uses it, is
//!   `Safe`: a request buffer written through every writer shape, a fresh op
//!   handed to `rd_kafka_q_enq` (its custody leaves the caller on every exit),
//!   a protocol string constructed and freed, an accessor and a borrowed
//!   message read;
//! * **refusing:** a misuse is a violation candidate or a refusal, never
//!   `Safe`: an owned buffer only written still leaks (a writer borrows), an
//!   op used or freed after it was enqueued, an unreleased protocol string, a
//!   borrowed message handed to a release, and a possibly-NULL op at the
//!   non-null consumption;
//! * **withdrawal:** without the module each accepting body has an
//!   uncontracted call again;
//! * **the header body in the unit:** a unit that also defines the header
//!   body (as every including unit does) and whose body does not validate
//!   keeps the declaration in force for its callers.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/text.rs"]
mod text;
use text::normalize_newlines;

const HEADER_INLINES: &str = include_str!(
    "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/header_inlines.qv"
);
const LIBC_MEMORY: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv");

const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ size_t; \
typedef signed char int8_t; \
typedef short int16_t; \
typedef int int32_t; \
typedef long long int64_t; \
typedef unsigned long long uint64_t; \
typedef unsigned char uint8_t; \
typedef unsigned int uint32_t; \
typedef uint8_t rd_bool_t; \
typedef struct rd_kafka_buf_s rd_kafka_buf_t; \
typedef struct rd_kafka_q_s rd_kafka_q_t; \
typedef struct rd_kafka_op_s rd_kafka_op_t; \
typedef struct rd_kafka_msgq_s rd_kafka_msgq_t; \
typedef struct rd_kafka_msg_s rd_kafka_msg_t; \
typedef struct rd_kafka_broker_s rd_kafka_broker_t; \
typedef struct rd_kafkap_str_s { int len; const char *str; } rd_kafkap_str_t; \
typedef enum { RD_KAFKA_BROKER_STATE_INIT, RD_KAFKA_BROKER_STATE_UP } rd_kafka_broker_state_t; \
void *malloc(size_t size); \
void free(void *ptr); \
size_t rd_kafka_buf_write(rd_kafka_buf_t *rkbuf, const void *data, size_t len); \
size_t rd_kafka_buf_write_i8(rd_kafka_buf_t *rkbuf, int8_t v); \
size_t rd_kafka_buf_write_i16(rd_kafka_buf_t *rkbuf, int16_t v); \
size_t rd_kafka_buf_write_i32(rd_kafka_buf_t *rkbuf, int32_t v); \
size_t rd_kafka_buf_write_i64(rd_kafka_buf_t *rkbuf, int64_t v); \
size_t rd_kafka_buf_write_bool(rd_kafka_buf_t *rkbuf, rd_bool_t v); \
size_t rd_kafka_buf_write_uvarint(rd_kafka_buf_t *rkbuf, uint64_t v); \
size_t rd_kafka_buf_write_arraycnt(rd_kafka_buf_t *rkbuf, size_t cnt); \
size_t rd_kafka_buf_write_str(rd_kafka_buf_t *rkbuf, const char *str, size_t len); \
size_t rd_kafka_buf_write_kstr(rd_kafka_buf_t *rkbuf, const rd_kafkap_str_t *kstr); \
void rd_kafka_buf_update_i32(rd_kafka_buf_t *rkbuf, size_t of, int32_t v); \
void rd_kafka_buf_finalize_arraycnt(rd_kafka_buf_t *rkbuf, size_t of, size_t cnt); \
rd_kafkap_str_t *rd_kafkap_str_new(const char *str, int len); \
int rd_kafka_q_enq(rd_kafka_q_t *rkq, rd_kafka_op_t *rko); \
int rd_kafka_msgq_enq(rd_kafka_msgq_t *rkmq, rd_kafka_msg_t *rkm); \
rd_kafka_msg_t *rd_kafka_msgq_first(const rd_kafka_msgq_t *rkmq); \
rd_kafka_broker_state_t rd_kafka_broker_get_state(rd_kafka_broker_t *rkb);\n";

fn production() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        (
            "librdkafka/header_inlines.qv".to_owned(),
            normalize_newlines(HEADER_INLINES),
        ),
    ]))
    .unwrap_or_else(|error| panic!("production contract text rejected: {error}"))
}

fn without_module() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "libc/memory.qv".to_owned(),
        normalize_newlines(LIBC_MEMORY),
    )]))
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
            eprintln!("skipping Clang-dependent LIBRDKAFKA-HEADER-INLINES test: {reason}");
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
    summary_of_source(contracts, &format!("{DECLARATIONS}{body}\n"), entry)
}

fn summary_of_source(
    contracts: &ResourceContracts,
    source: &str,
    entry: &str,
) -> LegacyFunctionSummary {
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
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

/// One accepting body per declaration family, each on what the caller owns
/// or borrows the way the corpus's callers do.
const ACCEPTING: [(&str, &str); 6] = [
    (
        "buffer writers",
        "void a(rd_kafka_buf_t *b, const rd_kafkap_str_t *k, const char *s) { size_t of; \
         rd_kafka_buf_write_i8(b, 1); rd_kafka_buf_write_i16(b, 2); rd_kafka_buf_write_bool(b, 1); \
         of = rd_kafka_buf_write_i32(b, 0); rd_kafka_buf_write_i64(b, 4); \
         rd_kafka_buf_write_arraycnt(b, 3); rd_kafka_buf_write_uvarint(b, 5); \
         rd_kafka_buf_write_str(b, s, (size_t)-1); rd_kafka_buf_write_kstr(b, k); \
         rd_kafka_buf_write(b, s, 2); rd_kafka_buf_update_i32(b, of, 7); \
         rd_kafka_buf_finalize_arraycnt(b, of, 7); }",
    ),
    (
        "op enqueue",
        "void a(rd_kafka_q_t *q) { rd_kafka_op_t *rko; rko = malloc(64); if (!rko) { return; } \
         rd_kafka_q_enq(q, rko); }",
    ),
    (
        "message enqueue",
        "void a(rd_kafka_msgq_t *mq) { rd_kafka_msg_t *m; m = malloc(64); if (!m) { return; } \
         rd_kafka_msgq_enq(mq, m); }",
    ),
    (
        "protocol string",
        "void a(rd_kafka_buf_t *b, const char *s) { rd_kafkap_str_t *k; k = rd_kafkap_str_new(s, -1); \
         if (!k) { return; } rd_kafka_buf_write_kstr(b, k); free(k); }",
    ),
    (
        "accessor",
        "int a(rd_kafka_broker_t *rkb) { rd_kafka_broker_state_t s; s = rd_kafka_broker_get_state(rkb); \
         return s == RD_KAFKA_BROKER_STATE_UP; }",
    ),
    (
        "borrowed message",
        "int a(const rd_kafka_msgq_t *mq) { rd_kafka_msg_t *m; m = rd_kafka_msgq_first(mq); \
         return m != 0; }",
    ),
];

#[test]
fn every_family_accepts_its_correct_use() {
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
fn without_the_module_every_accepting_body_has_an_uncontracted_call() {
    if !clang_available() {
        return;
    }
    let bare = without_module();
    for (name, body) in ACCEPTING {
        let summary = summary_with(&bare, body, "a");
        assert_ne!(
            summary.status,
            AnalysisStatus::Safe,
            "{name}: must not be Safe without the module"
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
fn a_writer_borrows_so_an_owned_buffer_only_written_still_leaks() {
    if !clang_available() {
        return;
    }
    for writer in [
        "rd_kafka_buf_write_i32(b, 1)",
        "rd_kafka_buf_write_str(b, \"x\", 1)",
        "rd_kafka_buf_finalize_arraycnt(b, 0, 1)",
    ] {
        let body = format!(
            "void l(void) {{ rd_kafka_buf_t *b; b = malloc(64); if (!b) {{ return; }} {writer}; }}"
        );
        let summary = summary(&body, "l");
        assert_eq!(
            summary.status,
            AnalysisStatus::Unsafe,
            "{writer}: {:?}",
            summary.diagnostics
        );
        assert_eq!(leaks(&summary), 1, "{writer}: {:?}", summary.diagnostics);
    }
}

#[test]
fn a_protocol_string_is_owned_by_its_caller() {
    if !clang_available() {
        return;
    }
    let summary = summary(
        "void l(const char *s) { rd_kafkap_str_t *k; k = rd_kafkap_str_new(s, -1); if (!k) { return; } }",
        "l",
    );
    assert_eq!(
        summary.status,
        AnalysisStatus::Unsafe,
        "{:?}",
        summary.diagnostics
    );
    assert_eq!(leaks(&summary), 1, "{:?}", summary.diagnostics);
}

#[test]
fn an_enqueued_op_is_no_longer_the_callers() {
    if !clang_available() {
        return;
    }
    let bodies = [
        // Freed after the queue took it.
        "void u(rd_kafka_q_t *q) { rd_kafka_op_t *rko; rko = malloc(64); if (!rko) { return; } \
         rd_kafka_q_enq(q, rko); free(rko); }",
        // Enqueued twice.
        "void u(rd_kafka_q_t *q) { rd_kafka_op_t *rko; rko = malloc(64); if (!rko) { return; } \
         rd_kafka_q_enq(q, rko); rd_kafka_q_enq(q, rko); }",
        // A possibly-NULL op at the non-null consumption.
        "void u(rd_kafka_q_t *q) { rd_kafka_op_t *rko; rko = malloc(64); rd_kafka_q_enq(q, rko); }",
        // A message freed after the queue took it.
        "void u(rd_kafka_msgq_t *mq) { rd_kafka_msg_t *m; m = malloc(64); if (!m) { return; } \
         rd_kafka_msgq_enq(mq, m); free(m); }",
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
fn a_borrowed_message_is_never_released_by_its_reader() {
    if !clang_available() {
        return;
    }
    let summary = summary(
        "void u(const rd_kafka_msgq_t *mq) { rd_kafka_msg_t *m; m = rd_kafka_msgq_first(mq); free(m); }",
        "u",
    );
    assert_ne!(
        summary.status,
        AnalysisStatus::Safe,
        "{:?}",
        summary.diagnostics
    );
}

/// Every including unit carries the header body. Here the unit defines
/// `rd_kafka_q_enq` the way `rdkafka_queue.h` does, forwarding to an
/// undeclared helper, so W4 validates no summary of it: the declaration
/// stays in force at the call and the caller is decided by it.
#[test]
fn a_header_body_that_does_not_validate_leaves_the_declaration_in_force() {
    if !clang_available() {
        return;
    }
    const PROTOTYPE: &str = "int rd_kafka_q_enq(rd_kafka_q_t *rkq, rd_kafka_op_t *rko); ";
    assert!(DECLARATIONS.contains(PROTOTYPE));
    let body = "int rd_kafka_q_enq1(rd_kafka_q_t *rkq, rd_kafka_op_t *rko, rd_kafka_q_t *orig, int at_head, int do_lock); \
        static inline int rd_kafka_q_enq(rd_kafka_q_t *rkq, rd_kafka_op_t *rko) { \
        return rd_kafka_q_enq1(rkq, rko, rkq, 0, 1); } \
        void a(rd_kafka_q_t *q) { rd_kafka_op_t *rko; rko = malloc(64); if (!rko) { return; } \
        rd_kafka_q_enq(q, rko); }";
    let source = format!("{}{body}\n", DECLARATIONS.replace(PROTOTYPE, ""));
    let summary = summary_of_source(&production(), &source, "a");
    assert_eq!(
        summary.status,
        AnalysisStatus::Safe,
        "{:?}",
        summary.diagnostics
    );
}

/// The corpus's `rd_kafka_share_fetch_fanout_enqueue` and
/// `rd_kafka_queue_yield` shapes: a formal op handed on to the queue (the
/// caller's custody passed through), a queue read from a member at a borrowed
/// position, and a formal buffer written.
#[test]
fn the_predicted_corpus_shapes_are_safe() {
    if !clang_available() {
        return;
    }
    for body in [
        "void f(rd_kafka_q_t *q, rd_kafka_op_t *rko) { rd_kafka_q_enq(q, rko); }",
        "struct rk { rd_kafka_q_t *ops; }; void f(struct rk *rk, rd_kafka_op_t *rko) { rd_kafka_q_enq(rk->ops, rko); }",
        "void f(rd_kafka_buf_t *b) { rd_kafka_buf_write_i8(b, 1); }",
    ] {
        let summary = summary(body, "f");
        assert_eq!(
            summary.status,
            AnalysisStatus::Safe,
            "{body}: {:?}",
            summary.diagnostics
        );
    }
}
