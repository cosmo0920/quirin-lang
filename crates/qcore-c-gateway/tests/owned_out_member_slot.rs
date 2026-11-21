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

//! CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT (gateway rule 72): an owned value a
//! callee delivers through an out slot handed a member path of an object the
//! caller supplied (`docs/reports/cg10-owned-out-parameter-member-slot/`).
//!
//! The declarations are the production provider modules, byte for byte; the
//! bodies are held-out fixtures of the 13 census shapes and of every refusal
//! the rule keeps. They are not claims about any upstream body.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/text.rs"]
mod text;
use text::normalize_newlines;

const LIBC_MEMORY: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv");
const CMETRICS: &str = include_str!(
    "../../../providers/qcore-provider-cmetrics/contracts/cmetrics/mpack_string_out.qv"
);
const CTRACES: &str =
    include_str!("../../../providers/qcore-provider-ctraces/contracts/ctraces/mpack_string_out.qv");
const RDKAFKA: &str = include_str!(
    "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/compress_out.qv"
);

const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ size_t; typedef char *cfl_sds_t; \
typedef struct mpack_reader_s mpack_reader_t; \
typedef struct rd_kafka_broker_s rd_kafka_broker_t; typedef struct rd_slice_s rd_slice_t; \
typedef enum { RD_KAFKA_RESP_ERR__BAD_MSG = -199, RD_KAFKA_RESP_ERR_NO_ERROR = 0 } rd_kafka_resp_err_t; \
void *malloc(size_t size); void free(void *ptr); \
int cmt_mpack_consume_string_tag(mpack_reader_t *reader, cfl_sds_t *output_buffer); \
int ctr_mpack_consume_string_or_nil_tag(mpack_reader_t *reader, cfl_sds_t *output_buffer); \
rd_kafka_resp_err_t rd_kafka_gzip_compress(rd_kafka_broker_t *rkb, int comp_level, rd_slice_t *slice, \
  void **outbuf, size_t *outlenp); \
rd_kafka_resp_err_t rd_kafka_snappy_compress_slice(rd_kafka_broker_t *rkb, rd_slice_t *slice, \
  void **outbuf, size_t *outlenp); \
rd_kafka_resp_err_t rd_kafka_lz4_compress(rd_kafka_broker_t *rkb, int proper_hc, int comp_level, \
  rd_slice_t *slice, void **outbuf, size_t *outlenp); \
struct cmt_opts { cfl_sds_t ns; cfl_sds_t subsystem; }; struct cmt_map { struct cmt_opts *opts; }; \
struct cmt_label { cfl_sds_t key; cfl_sds_t val; }; \
struct scope { cfl_sds_t name; }; struct status { int code; cfl_sds_t message; }; \
struct span { struct status status; cfl_sds_t schema_url; }; \
struct scope_span { struct scope *instrumentation_scope; }; \
struct decode_context { struct span *span; struct scope_span *scope_span; }; \
struct iovec { void *iov_base; size_t iov_len; }; \
struct msetw { rd_kafka_broker_t *msetw_rkb; int msetw_MsgVersion; int level; };\n";

fn production() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        (
            "cmetrics/mpack_string_out.qv".to_owned(),
            normalize_newlines(CMETRICS),
        ),
        (
            "ctraces/mpack_string_out.qv".to_owned(),
            normalize_newlines(CTRACES),
        ),
        (
            "librdkafka/compress_out.qv".to_owned(),
            normalize_newlines(RDKAFKA),
        ),
    ]))
    .unwrap_or_else(|error| panic!("production contract text rejected: {error}"))
}

/// Only libc's allocator: the card's declarations are absent.
fn without_card() -> ResourceContracts {
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
            eprintln!("skipping Clang-dependent control: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    descriptions: Vec<String>,
}

fn run_named(source: &str, function: &str, contracts: &ResourceContracts) -> Run {
    let text = format!("{DECLARATIONS}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        function,
        contracts,
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    let result = summary
        .and_then(|summary| summary.functions.get(function))
        .unwrap_or_else(|| panic!("`{function}` projected: {outcome:?}"));
    Run {
        status: result.status,
        descriptions: result
            .diagnostics
            .iter()
            .map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
    }
}

fn run(source: &str, contracts: &ResourceContracts) -> Run {
    run_named(source, "subject", contracts)
}

const GATE: &str = "indirect memory access is outside the resource projection";

/// The 13 census shapes (round 4's `rank-tip.json`), one fixture per form.
const ACCEPTED: &[&str] = &[
    // cmetrics `unpack_opts_ns` (and `_ss`, `_name`, `_desc`).
    "int subject(mpack_reader_t *reader, size_t index, void *context) { \
       struct cmt_map *map; struct cmt_opts *opts; \
       map = (struct cmt_map *) context; opts = map->opts; \
       return cmt_mpack_consume_string_tag(reader, &opts->ns); }",
    // cmetrics `unpack_static_label_component`.
    "int subject(mpack_reader_t *reader, size_t index, void *context) { \
       struct cmt_label *label; \
       if (0 == reader || 0 == context) { return -1; } \
       label = (struct cmt_label *) context; \
       if (index == 0) { return cmt_mpack_consume_string_tag(reader, &label->key); } \
       else if (index == 1) { return cmt_mpack_consume_string_tag(reader, &label->val); } \
       return -1; }",
    // ctraces `unpack_instrumentation_scope_name` (and `_version`).
    "int subject(mpack_reader_t *reader, size_t index, void *ctx) { \
       struct decode_context *context = ctx; \
       return ctr_mpack_consume_string_or_nil_tag(reader, \
         &context->scope_span->instrumentation_scope->name); }",
    // ctraces `unpack_span_status_message`.
    "int subject(mpack_reader_t *reader, size_t index, void *ctx) { \
       struct decode_context *context = ctx; \
       return ctr_mpack_consume_string_or_nil_tag(reader, &context->span->status.message); }",
    // ctraces `unpack_span_schema_url` (and `unpack_link_trace_state`).
    "int subject(mpack_reader_t *reader, size_t index, void *ctx) { \
       struct decode_context *context = ctx; \
       return ctr_mpack_consume_string_or_nil_tag(reader, &context->span->schema_url); }",
    // librdkafka `rd_kafka_msgset_writer_compress_gzip`.
    "int subject(struct msetw *msetw, rd_slice_t *slice, struct iovec *ciov) { \
       rd_kafka_resp_err_t err; int comp_level = msetw->level; \
       err = rd_kafka_gzip_compress(msetw->msetw_rkb, comp_level, slice, \
         &ciov->iov_base, &ciov->iov_len); \
       return (err ? -1 : 0); }",
    // librdkafka `rd_kafka_msgset_writer_compress_snappy`.
    "int subject(struct msetw *msetw, rd_slice_t *slice, struct iovec *ciov) { \
       rd_kafka_resp_err_t err; \
       err = rd_kafka_snappy_compress_slice(msetw->msetw_rkb, slice, \
         &ciov->iov_base, &ciov->iov_len); \
       return (err ? -1 : 0); }",
    // librdkafka `rd_kafka_msgset_writer_compress_lz4`.
    "int subject(struct msetw *msetw, rd_slice_t *slice, struct iovec *ciov) { \
       rd_kafka_resp_err_t err; int comp_level = msetw->level; \
       err = rd_kafka_lz4_compress(msetw->msetw_rkb, msetw->msetw_MsgVersion >= 1 ? 1 : 0, \
         comp_level, slice, &ciov->iov_base, &ciov->iov_len); \
       return (err ? -1 : 0); }",
];

#[test]
fn each_census_shape_delivers_into_the_callers_object() {
    if !clang_available() {
        return;
    }
    for body in ACCEPTED {
        let run = run(body, &production());
        assert_eq!(run.status, AnalysisStatus::Safe, "{body}\n{run:?}");
    }
}

#[test]
fn without_the_declarations_each_shape_keeps_a_refusal() {
    if !clang_available() {
        return;
    }
    for body in ACCEPTED {
        let run = run(body, &without_card());
        assert_eq!(run.status, AnalysisStatus::Unknown, "{body}\n{run:?}");
    }
}

/// Every form rule 72 refuses keeps a refusal.
const REFUSED: &[(&str, &str)] = &[
    (
        "a record this body allocated (a tracked owner: O04-B08's field link)",
        "int subject(mpack_reader_t *reader) { struct cmt_label *l = malloc(sizeof *l); \
           if (!l) return -1; cmt_mpack_consume_string_tag(reader, &l->key); free(l); return 0; }",
    ),
    (
        "a pointer to this frame's own object",
        "int subject(mpack_reader_t *reader) { struct cmt_label v; struct cmt_label *p; \
           p = &v; return cmt_mpack_consume_string_tag(reader, &p->key); }",
    ),
    (
        "a rebound formal",
        "int subject(mpack_reader_t *reader, struct cmt_label *l, struct cmt_label *m) { \
           l = m; return cmt_mpack_consume_string_tag(reader, &l->key); }",
    ),
    (
        "an indexed slot",
        "int subject(mpack_reader_t *reader, struct cmt_label *l) { \
           return cmt_mpack_consume_string_tag(reader, &(&l[1])->key); }",
    ),
    (
        "the call inside a larger expression",
        "int subject(mpack_reader_t *reader, struct cmt_label *l) { \
           return 1 + cmt_mpack_consume_string_tag(reader, &l->key); }",
    ),
    (
        "a member of this frame's own record object",
        "int subject(mpack_reader_t *reader) { struct cmt_label v; \
           return cmt_mpack_consume_string_tag(reader, &v.key); }",
    ),
];

#[test]
fn every_other_slot_keeps_a_refusal() {
    if !clang_available() {
        return;
    }
    for (why, body) in REFUSED {
        let run = run(body, &production());
        assert_ne!(run.status, AnalysisStatus::Safe, "{why}: {body}\n{run:?}");
    }
}

#[test]
fn a_delivery_does_not_launder_a_frame_leak() {
    if !clang_available() {
        return;
    }
    // A buffer this body allocates and drops still leaks beside the delivery.
    let run = run(
        "int subject(mpack_reader_t *reader, struct cmt_label *l) { char *b = malloc(8); \
           if (!b) return -1; return cmt_mpack_consume_string_tag(reader, &l->key); }",
        &production(),
    );
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
}

#[test]
fn a_body_with_a_delivery_publishes_no_summary() {
    if !clang_available() {
        return;
    }
    // The caller of a rule-72 body sees an uncontracted callee: the summary
    // would read the formal `Borrowed` although its member now owns custody.
    let source = "static int fill(mpack_reader_t *reader, struct cmt_label *l) { \
           return cmt_mpack_consume_string_tag(reader, &l->key); } \
         int subject(mpack_reader_t *reader, struct cmt_label *l) { return fill(reader, l); }";
    let caller = run(source, &production());
    assert_eq!(caller.status, AnalysisStatus::Unknown, "{caller:?}");
    assert!(
        caller
            .descriptions
            .iter()
            .any(|text| text == "call to uncontracted function `fill`"),
        "{caller:?}"
    );
    let filler = run_named(source, "fill", &production());
    assert_eq!(filler.status, AnalysisStatus::Safe, "{filler:?}");
}

#[test]
fn a_local_slot_keeps_the_return_code_reading() {
    if !clang_available() {
        return;
    }
    // C04.13 unchanged: a tested `&NAME` slot owes its release on the `0`
    // edge; an untested one is refused (`relational-untested`).
    let tested = run(
        "int subject(mpack_reader_t *reader) { cfl_sds_t s; \
           if (cmt_mpack_consume_string_tag(reader, &s) != 0) return 1; free(s); return 0; }",
        &production(),
    );
    assert_eq!(tested.status, AnalysisStatus::Safe, "{tested:?}");
    let leaked = run(
        "int subject(mpack_reader_t *reader) { cfl_sds_t s; \
           if (cmt_mpack_consume_string_tag(reader, &s) != 0) return 1; return 0; }",
        &production(),
    );
    assert_eq!(leaked.status, AnalysisStatus::Unsafe, "{leaked:?}");
    let untested = run(
        "int subject(mpack_reader_t *reader) { cfl_sds_t s; \
           cmt_mpack_consume_string_tag(reader, &s); return 0; }",
        &production(),
    );
    assert_ne!(untested.status, AnalysisStatus::Safe, "{untested:?}");
}

#[test]
fn the_refusal_text_is_the_gates() {
    if !clang_available() {
        return;
    }
    let run = run(REFUSED[1].1, &production());
    assert!(run.descriptions.iter().any(|text| text == GATE), "{run:?}");
}
