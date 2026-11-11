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

//! W48-RK-LOG-CALLER-FRAME (gateway rule 32a): a caller-frame declaration
//! serves only callers outside its own frame
//! (`docs/reports/w48-rk-log-caller-frame/README.md`).
//!
//! The production declarations (`librdkafka/log_caller_frame.qv`) over a
//! synthetic logger pair shaped like librdkafka's: `rd_kafka_log_buf` (static,
//! an uncontracted enqueue standing for the logger's Open internal effects)
//! and `rd_kafka_log0`, which calls it. The regression: `rd_kafka_log0`'s own
//! row read its sibling's caller-frame declaration and became `Safe` on the
//! frame's own declaration. Each member's row must be decided by its body,
//! while a caller outside the frame still reads the declaration.
//!
//! `Safe` here is `Safe` for a synthetic declared projection under the
//! declarations' three named premises, never a statement about librdkafka.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, LegacyDiagnosticKind, LegacyFunctionSummary, LegacyProgramSummary,
    ResourceContracts, analyze_legacy_c_translation_unit, caller_frame_hold,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::reason::ReasonCategory;

const DECLARATIONS: &str = include_str!(
    "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/log_caller_frame.qv"
);

const PRELUDE: &str = r#"
typedef struct rd_kafka_conf_s { int log_level; int log_queue; } rd_kafka_conf_t;
typedef struct rd_kafka_s { void *rk_logq; } rd_kafka_t;
void rd_kafka_q_enq_fixture(void *q, const char *buf);
"#;

/// The frame's own bodies, with an outside caller in the same unit.
const BODIES: &str = r#"
static void rd_kafka_log_buf(const rd_kafka_conf_t *conf, const rd_kafka_t *rk, int level,
                             int ctx, const char *fac, const char *buf)
{
    if (level > conf->log_level) {
        return;
    }
    rd_kafka_q_enq_fixture(rk->rk_logq, buf);
}

void rd_kafka_log0(const rd_kafka_conf_t *conf, const rd_kafka_t *rk, const char *extra,
                   int level, int ctx, const char *fac, const char *fmt, ...)
{
    if (level > conf->log_level) {
        return;
    }
    rd_kafka_log_buf(conf, rk, level, ctx, fac, fmt);
}

void outside_caller(const rd_kafka_conf_t *conf, const rd_kafka_t *rk)
{
    rd_kafka_log0(conf, rk, 0, 7, 0, "FAC", "plain");
}
"#;

/// A frame member whose only blocker would be its sibling's declaration: its
/// own body has no other effect.
const TRIVIAL_SIBLING: &str = r#"
static void rd_kafka_log_buf(const rd_kafka_conf_t *conf, const rd_kafka_t *rk, int level,
                             int ctx, const char *fac, const char *buf)
{
    return;
}

void rd_kafka_log0(const rd_kafka_conf_t *conf, const rd_kafka_t *rk, const char *extra,
                   int level, int ctx, const char *fac, const char *fmt, ...)
{
    rd_kafka_log_buf(conf, rk, level, ctx, fac, fmt);
}
"#;

/// A member calling itself: its own declaration never serves its own body.
const RECURSIVE: &str = r#"
void rd_kafka_log0(const rd_kafka_conf_t *conf, const rd_kafka_t *rk, const char *extra,
                   int level, int ctx, const char *fac, const char *fmt, ...)
{
    if (level <= 0) {
        return;
    }
    rd_kafka_log0(conf, rk, extra, level - 1, ctx, fac, fmt);
}
"#;

/// A caller outside the frame, in a unit without the frame's bodies.
const CALLER_ONLY: &str = r#"
void rd_kafka_log0(const rd_kafka_conf_t *conf, const rd_kafka_t *rk, const char *extra,
                   int level, int ctx, const char *fac, const char *fmt, ...);

void outside_caller(const rd_kafka_conf_t *conf, const rd_kafka_t *rk)
{
    rd_kafka_log0(conf, rk, 0, 7, 0, "FAC", "plain");
}
"#;

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "log_caller_frame.qv".to_owned(),
        DECLARATIONS.to_owned(),
    )]))
    .expect("the production caller-frame declarations load")
}

fn analyse(source: &str, entry: &str, contracts: &ResourceContracts) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(&format!("{PRELUDE}\n{source}"), entry, contracts)
        .expect("the fixture analyses")
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

fn descriptions(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description }
            | LegacyDiagnosticKind::CountedUser { description } => description.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

fn held(summary: &LegacyFunctionSummary, callee: &str) -> bool {
    descriptions(summary)
        .iter()
        .any(|description| caller_frame_hold::held_call(description) == Some(callee))
}

#[test]
fn a_member_row_is_decided_by_its_body_not_by_the_frame_declaration() {
    let summary = analyse(BODIES, "rd_kafka_log0", &contracts());
    let log0 = function(&summary, "rd_kafka_log0");
    assert_eq!(log0.status, AnalysisStatus::Unknown, "{log0:#?}");
    assert!(held(log0, "rd_kafka_log_buf"), "{:#?}", descriptions(log0));
    let log_buf = function(&summary, "rd_kafka_log_buf");
    assert_ne!(log_buf.status, AnalysisStatus::Safe, "{log_buf:#?}");
}

#[test]
fn a_caller_outside_the_frame_still_reads_the_declaration() {
    let contracts = contracts();
    let same_unit = analyse(BODIES, "outside_caller", &contracts);
    let caller = function(&same_unit, "outside_caller");
    assert_eq!(caller.status, AnalysisStatus::Safe, "{caller:#?}");
    assert!(!held(caller, "rd_kafka_log0"));
    let other_unit = analyse(CALLER_ONLY, "outside_caller", &contracts);
    let caller = function(&other_unit, "outside_caller");
    assert_eq!(caller.status, AnalysisStatus::Safe, "{caller:#?}");
}

#[test]
fn a_sibling_declaration_never_certifies_a_member_even_when_nothing_else_blocks() {
    let summary = analyse(TRIVIAL_SIBLING, "rd_kafka_log0", &contracts());
    let log0 = function(&summary, "rd_kafka_log0");
    assert_eq!(log0.status, AnalysisStatus::Unknown, "{log0:#?}");
    assert!(held(log0, "rd_kafka_log_buf"));
}

#[test]
fn a_member_own_declaration_never_serves_its_recursive_call() {
    let summary = analyse(RECURSIVE, "rd_kafka_log0", &contracts());
    let log0 = function(&summary, "rd_kafka_log0");
    assert_eq!(log0.status, AnalysisStatus::Unknown, "{log0:#?}");
    assert!(held(log0, "rd_kafka_log0"));
}

#[test]
fn without_the_declarations_nothing_is_held() {
    let empty = ResourceContracts::default();
    let summary = analyse(BODIES, "rd_kafka_log0", &empty);
    for row in summary.functions.values() {
        assert!(!held(row, "rd_kafka_log_buf") && !held(row, "rd_kafka_log0"));
    }
}

#[test]
fn a_source_spelled_hold_prefix_is_escaped_and_refuses() {
    let source = r#"
void qcore_caller_frame_self__rd_kafka_log0(const rd_kafka_conf_t *conf);

void forged(const rd_kafka_conf_t *conf)
{
    qcore_caller_frame_self__rd_kafka_log0(conf);
}
"#;
    let summary = analyse(source, "forged", &contracts());
    let forged = function(&summary, "forged");
    assert_eq!(forged.status, AnalysisStatus::Unknown, "{forged:#?}");
    assert!(
        !held(forged, "rd_kafka_log0"),
        "{:#?}",
        descriptions(forged)
    );
}

#[test]
fn the_hold_is_unknown_with_an_analyser_category_and_round_trips() {
    let description = caller_frame_hold::hold_description("rd_kafka_log_buf");
    assert_eq!(
        caller_frame_hold::held_call(&description),
        Some("rd_kafka_log_buf")
    );
    assert_eq!(
        ReasonCategory::classify(&description),
        Ok(ReasonCategory::IncompleteObligationCoverage)
    );
    assert!(!ReasonCategory::IncompleteObligationCoverage.describes_analysed_source());
}
