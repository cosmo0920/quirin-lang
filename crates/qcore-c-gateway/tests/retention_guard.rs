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

//! Card W43-ABI-ENUM-AND-CONST-PP (gateway rule 12w): a guarded retention
//! (`resource_retained_by(src, state) when kind == 2`, contract specification
//! §C04.14.1) on route G. A decided actual that fails the guard drops the
//! clause at that call; a decided success, an undecided actual (a variable, a
//! value out of the parameter's range, a shadowed enumeration constant) keeps
//! it, the cautious reading; in a unit that defines the callee, the guard
//! stands only when the body validates it. Synthetic trusted assumptions: a
//! `Safe` here is `Safe` over them and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACT: &str = "module contract;

extern fn g_set(state: ptr<mut, u8>, kind: u32, src: ptr<const, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(state);
    ensures normal resource_borrowed(kind);
    ensures normal resource_borrowed(src);
    ensures normal resource_retained_by(src, state) when kind == 2;
    ensures normal resource_return_none();
};
";

const PRELUDE: &str = "\
struct st { const void *kept; int n; };
enum kinds { K_INT, K_STR, K_PTR };
int g_set(struct st *state, unsigned kind, const void *src);
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "guarded.qcore".to_owned(),
        CONTRACT.to_owned(),
    )]))
    .expect("the fixture contract loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent W43-ABI-ENUM-AND-CONST-PP test: {reason}");
            false
        }
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// Every row of `PRELUDE + source`: status and unsupported descriptions.
fn rows(source: &str, entry: &str) -> BTreeMap<String, (AnalysisStatus, Vec<String>)> {
    let text = format!("{PRELUDE}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        entry,
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    summary
        .functions
        .iter()
        .map(|(name, row)| {
            (
                name.clone(),
                (
                    row.status,
                    row.diagnostics
                        .iter()
                        .filter_map(|d| match &d.kind {
                            LegacyDiagnosticKind::Unsupported { description } => {
                                Some(description.clone())
                            }
                            _ => None,
                        })
                        .collect(),
                ),
            )
        })
        .collect()
}

const CALLERS: &str = "\
int decided_off(struct st *state, const void *src) { return g_set(state, 1, src); }
int decided_on(struct st *state, const void *src) { return g_set(state, 2, src); }
int decided_off_suffixed(struct st *state, const void *src) { return g_set(state, (0u), src); }
int undecided(struct st *state, const void *src, unsigned k) { return g_set(state, k, src); }
int out_of_range(struct st *state, const void *src) { return g_set(state, -1, src); }
int enum_off(struct st *state, const void *src) { return g_set(state, K_INT, src); }
int enum_on(struct st *state, const void *src) { return g_set(state, K_PTR, src); }
int shadowed(struct st *state, const void *src) { unsigned K_INT = 2; return g_set(state, K_INT, src); }
";

fn safe(rows: &BTreeMap<String, (AnalysisStatus, Vec<String>)>, name: &str) -> bool {
    rows[name].0 == AnalysisStatus::Safe
}

fn retention_undecided(rows: &BTreeMap<String, (AnalysisStatus, Vec<String>)>, name: &str) -> bool {
    rows[name].0 != AnalysisStatus::Safe
        && rows[name]
            .1
            .iter()
            .any(|description| description.contains("retention-undecided"))
}

#[test]
fn a_decided_failure_drops_the_clause_and_everything_else_keeps_it() {
    if !clang_available() {
        return;
    }
    let rows = rows(CALLERS, "decided_off");
    for name in ["decided_off", "decided_off_suffixed", "enum_off"] {
        assert!(safe(&rows, name), "{name}: {:?}", rows[name]);
    }
    // The caller supplied the holder: a retention that stands is undecided.
    for name in [
        "decided_on",
        "undecided",
        "out_of_range",
        "enum_on",
        "shadowed",
    ] {
        assert!(retention_undecided(&rows, name), "{name}: {:?}", rows[name]);
    }
}

#[test]
fn a_body_in_the_unit_validates_its_guard_or_the_cautious_reading_stands() {
    if !clang_available() {
        return;
    }
    let validating = "\
int g_set(struct st *state, unsigned kind, const void *src) {
    if (!src) return -1;
    switch (kind) {
    case K_PTR: state->kept = src; break;
    case K_INT: state->n = *(const int *)src; break;
    default: return -1;
    }
    return 0;
}
";
    let validated = rows(&format!("{validating}{CALLERS}"), "decided_off");
    assert!(
        safe(&validated, "decided_off"),
        "{:?}",
        validated["decided_off"]
    );
    assert!(
        retention_undecided(&validated, "decided_on"),
        "{:?}",
        validated["decided_on"]
    );
    // The retention depends on the holder's own state, not the argument
    // (the shape `rd_kafka_confval_set_type` has): not validated, so the
    // cautious reading stands at every call of this unit.
    let holder_switched = "\
int g_set(struct st *state, unsigned kind, const void *src) {
    switch (state->n) {
    case 2: state->kept = src; break;
    default: break;
    }
    return kind == 2 ? 0 : -1;
}
";
    let switched = rows(&format!("{holder_switched}{CALLERS}"), "decided_off");
    for name in ["decided_off", "enum_off"] {
        assert!(
            retention_undecided(&switched, name),
            "{name}: {:?}",
            switched[name]
        );
    }
}
