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

//! W46-METRIC-TRACE-CONTRACT-GAPS: the selected owner declarations are premises.
//! These controls exercise the gateway around those premises, not the C bodies.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const METRIC: &str =
    include_str!("../../../providers/qcore-provider-cmetrics/contracts/cmetrics/metric_values.qv");
const TRACE: &str =
    include_str!("../../../providers/qcore-provider-ctraces/contracts/ctraces/context_lifetime.qv");
const FIXTURE: &str = "module contract;\n\
extern fn w_open() -> ptr<mut, u8> contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_return_owned(nonnull);\n\
};\n\
extern fn w_close(p: ptr<mut, u8>) -> unit contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_released(p, nonnull);\n\
    ensures normal resource_return_none();\n\
};\n";
const HEADERS: &str = "void *w_open(void); void w_close(void *p); \
double cmt_metric_get_value(void *metric); \
void cmt_metric_set_timestamp(void *metric, __UINT64_TYPE__ timestamp); \
__SIZE_TYPE__ ctr_id_get_len(void *cid);";
// The trace span setter is declared separately so its scalar field write is
// exercised in the accepting and ownership controls below.
const SPAN_HEADER: &str =
    " void ctr_span_set_dropped_events_count(void *span, unsigned int count);";

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(std::env::var_os("QCORE_CLANG").is_none(), "{reason}");
            eprintln!("skipping Clang-dependent metric/trace test: {reason}");
            false
        }
    }
}

fn contracts(metric: &str, trace: &str) -> qcore_c_gateway::legacy_c::ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("metric.qcore".to_owned(), metric.to_owned()),
        ("trace.qcore".to_owned(), trace.to_owned()),
        ("fixture.qcore".to_owned(), FIXTURE.to_owned()),
    ]))
    .expect("the actual owner contract modules load")
}

fn analyze(body: &str, metric: &str, trace: &str) -> UnrestrictedLegacyResult {
    let source = format!("{HEADERS}{SPAN_HEADER} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(metric, trace),
    )
    .unwrap_or_else(|error| panic!("fixture rejected: {error:?}\n{source}"))
}

fn summary(result: &UnrestrictedLegacyResult) -> &LegacyFunctionSummary {
    let program: &LegacyProgramSummary = match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(program)
        | UnrestrictedLegacyResult::Unknown {
            partial: Some(program),
            ..
        } => program,
        other => panic!("expected a projected result: {other:?}"),
    };
    program
        .functions
        .get("w")
        .expect("entry function projected")
}

fn unsupported(result: &UnrestrictedLegacyResult) -> Vec<String> {
    summary(result)
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect()
}

fn without_declaration(module: &str, name: &str) -> String {
    let start = module
        .find(&format!("extern fn {name}("))
        .expect("the production declaration exists");
    let end = start + module[start..].find("};").expect("declaration end") + 2;
    format!("{}{}", &module[..start], &module[end..])
}

#[test]
fn selected_metric_and_trace_calls_are_admitted_and_withdrawal_restores_refusal() {
    if !clang_available() {
        return;
    }
    let body = "void w(void) { void *p = w_open(); if (!p) return; \
        cmt_metric_set_timestamp(p, 4); \
        double x = cmt_metric_get_value(p); \
        __SIZE_TYPE__ n = ctr_id_get_len(p); \
        ctr_span_set_dropped_events_count(p, 2); \
        (void)x; (void)n; w_close(p); }";
    let admitted = analyze(body, METRIC, TRACE);
    let UnrestrictedLegacyResult::Safe(certificate) = admitted else {
        panic!("selected calls should be admitted: {admitted:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("certificate rechecks");

    for (metric, trace, name) in [
        (
            without_declaration(METRIC, "cmt_metric_get_value"),
            TRACE.to_owned(),
            "cmt_metric_get_value",
        ),
        (
            METRIC.to_owned(),
            without_declaration(TRACE, "ctr_id_get_len"),
            "ctr_id_get_len",
        ),
        (
            METRIC.to_owned(),
            without_declaration(TRACE, "ctr_span_set_dropped_events_count"),
            "ctr_span_set_dropped_events_count",
        ),
    ] {
        let withdrawn = analyze(body, &metric, &trace);
        let expected = format!("call to uncontracted function `{name}`");
        assert!(
            unsupported(&withdrawn)
                .iter()
                .any(|reason| reason == &expected),
            "withdrawing {name} must restore its refusal: {withdrawn:?}"
        );
    }
}

#[test]
fn borrowed_scalar_write_does_not_discharge_ownership_or_accept_a_released_actual() {
    if !clang_available() {
        return;
    }
    let leaked = analyze(
        "void w(void) { void *p = w_open(); if (!p) return; cmt_metric_set_timestamp(p, 1); }",
        METRIC,
        TRACE,
    );
    assert!(
        summary(&leaked)
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "a borrowed write must retain the caller's obligation: {leaked:?}"
    );

    let released = analyze(
        "void w(void) { void *p = w_open(); if (!p) return; w_close(p); cmt_metric_get_value(p); }",
        METRIC,
        TRACE,
    );
    assert!(
        !matches!(released, UnrestrictedLegacyResult::Safe(_)),
        "a released borrowed actual must refuse"
    );
    assert_eq!(summary(&released).status, AnalysisStatus::Unknown);
    assert!(
        !summary(&released).diagnostics.is_empty(),
        "the refusal must remain visible: {released:?}"
    );
}
