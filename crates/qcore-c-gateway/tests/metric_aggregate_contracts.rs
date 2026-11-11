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

//! W46-METRIC-AGGREGATE-CONTRACTS: actual owner text and fail-closed controls.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const METRIC: &str =
    include_str!("../../../providers/qcore-provider-cmetrics/contracts/cmetrics/metric_values.qv");
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
__UINT64_TYPE__ cmt_metric_hist_get_value(void *metric, int bucket_id); \
__UINT64_TYPE__ cmt_metric_hist_get_count_value(void *metric); \
double cmt_metric_hist_get_sum_value(void *metric); \
double cmt_summary_quantile_get_value(void *metric, int quantile_id); \
double cmt_summary_get_sum_value(void *metric); \
__UINT64_TYPE__ cmt_summary_get_count_value(void *metric);";
const NAMES: [&str; 6] = [
    "cmt_metric_hist_get_value",
    "cmt_metric_hist_get_count_value",
    "cmt_metric_hist_get_sum_value",
    "cmt_summary_quantile_get_value",
    "cmt_summary_get_sum_value",
    "cmt_summary_get_count_value",
];

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(std::env::var_os("QCORE_CLANG").is_none(), "{reason}");
            eprintln!("skipping Clang-dependent metric aggregate test: {reason}");
            false
        }
    }
}

fn analyze(body: &str, metric: &str) -> UnrestrictedLegacyResult {
    let contracts = load_resource_contract_sources(&BTreeMap::from([
        ("metric.qcore".to_owned(), metric.to_owned()),
        ("fixture.qcore".to_owned(), FIXTURE.to_owned()),
    ]))
    .expect("actual owner module loads");
    let source = format!("{HEADERS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts,
    )
    .unwrap_or_else(|error| panic!("fixture rejected: {error:?}\n{source}"))
}

fn without_declaration(name: &str) -> String {
    let start = METRIC
        .find(&format!("extern fn {name}("))
        .expect("production declaration exists");
    let end = start + METRIC[start..].find("};").expect("declaration end") + 2;
    format!("{}{}", &METRIC[..start], &METRIC[end..])
}

fn diagnostics(result: &UnrestrictedLegacyResult) -> Vec<LegacyDiagnosticKind> {
    let program = match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(program)
        | UnrestrictedLegacyResult::Unknown {
            partial: Some(program),
            ..
        } => program,
        other => panic!("expected projected result: {other:?}"),
    };
    program.functions["w"]
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.kind.clone())
        .collect()
}

#[test]
fn six_calls_admit_then_each_withdrawal_restores_named_refusal() {
    if !clang_available() {
        return;
    }
    let body = "void w(void) { void *p = w_open(); if (!p) return; \
        __UINT64_TYPE__ a = cmt_metric_hist_get_value(p, 0); \
        __UINT64_TYPE__ b = cmt_metric_hist_get_count_value(p); \
        double c = cmt_metric_hist_get_sum_value(p); \
        double d = cmt_summary_quantile_get_value(p, 0); \
        double e = cmt_summary_get_sum_value(p); \
        __UINT64_TYPE__ f = cmt_summary_get_count_value(p); \
        (void)a; (void)b; (void)c; (void)d; (void)e; (void)f; w_close(p); }";
    let accepted = analyze(body, METRIC);
    let UnrestrictedLegacyResult::Safe(certificate) = accepted else {
        panic!("six borrowed scalar reads should admit: {accepted:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("certificate rechecks");

    for name in NAMES {
        let withdrawn = analyze(body, &without_declaration(name));
        let expected = format!("call to uncontracted function `{name}`");
        assert!(
            diagnostics(&withdrawn).iter().any(|kind| matches!(kind,
                LegacyDiagnosticKind::Unsupported { description } if description == &expected)),
            "withdrawing {name} must refuse: {withdrawn:?}"
        );
    }
}

#[test]
fn borrowed_aggregate_read_preserves_owned_duty_and_released_refusal() {
    if !clang_available() {
        return;
    }
    let leaked = analyze(
        "void w(void) { void *p = w_open(); if (!p) return; cmt_metric_hist_get_count_value(p); }",
        METRIC,
    );
    assert!(
        diagnostics(&leaked)
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
    );

    let released = analyze(
        "void w(void) { void *p = w_open(); if (!p) return; w_close(p); cmt_summary_get_count_value(p); }",
        METRIC,
    );
    assert!(!matches!(released, UnrestrictedLegacyResult::Safe(_)));
    assert!(!diagnostics(&released).is_empty());
}
