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

//! Card RETENTION-NULL-CONSTANT-ACTUAL, carried by OPENSSL-UNSTATED-CALLBACK-SLOTS
//! (gateway rule 12t, amended in place): a null pointer constant at a
//! retained position designates no object, so the call retains nothing and
//! decides whatever the holder; every other actual, a local, a formal or a
//! call result that may be null included, is decided exactly as before.
//!
//! The declarations are CALLEE-RETENTION-CLASS1's synthetic `r_*` fixtures
//! (`fixtures/contracts/retained_by.qv`): a `Safe` here is `Safe` over them
//! and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, UnrestrictedLegacyResult,
    analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
struct st { const char *base; int n; };
void *r_alloc(__SIZE_TYPE__ n); void r_free(void *p);
int r_keep(struct st *state, const char *src); void r_reset(struct st *state);
void r_global(const char *value);
const char *pick(void);
";

const SOURCE: &str = "\
int null_void_cast(struct st *state) { return r_keep(state, ((void *) 0)); }
int null_zero(struct st *state) { return r_keep(state, 0); }
int null_typed_cast(struct st *state) { return r_keep(state, (const char *) 0); }
void null_global(void) { r_global(((void *) 0)); }
void null_tracked(void) { struct st *h = r_alloc(16); if (!h) return; r_keep(h, 0); r_free(h); }
int local_maybe_null(struct st *state) { const char *p = 0; return r_keep(state, p); }
int formal_maybe_null(struct st *state, const char *src) { return r_keep(state, src); }
int frame_call_result(void) { struct st s; return r_keep(&s, pick()); }
int not_structural(struct st *state) { return r_keep(state, (const char *) (1 - 1)); }
int non_null_cast(struct st *state) { return r_keep(state, (const char *) 1); }
";

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent retention null constant test: {reason}");
            false
        }
    }
}

/// `(status, unsupported descriptions)` of every row of the fixture unit.
fn rows() -> BTreeMap<String, (AnalysisStatus, Vec<String>)> {
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "retained_by.qcore".to_owned(),
        include_str!("fixtures/contracts/retained_by.qv").to_owned(),
    )]))
    .expect("the fixture contracts load");
    let text = format!("{PRELUDE}\n{SOURCE}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "null_void_cast",
        &contracts,
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("a partial summary")
        }
    };
    summary
        .functions
        .iter()
        .map(|(name, row)| {
            let unsupported = row
                .diagnostics
                .iter()
                .filter_map(|diagnostic| match &diagnostic.kind {
                    LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                    _ => None,
                })
                .collect();
            (name.clone(), (row.status, unsupported))
        })
        .collect()
}

/// A null constant retains nothing: in a holder the caller supplied, in the
/// `global` holder and in a tracked holder alike, the call decides.
#[test]
fn a_null_constant_retains_nothing() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    for name in [
        "null_void_cast",
        "null_zero",
        "null_typed_cast",
        "null_global",
        "null_tracked",
    ] {
        let (status, leaves) = &rows[name];
        assert_eq!(*status, AnalysisStatus::Safe, "{name}: {leaves:?}");
    }
}

/// Every other actual is decided as before: a local or a formal that may be
/// null, a call's result, an integer expression the reading does not
/// evaluate and a non-null cast all keep their refusal.
#[test]
fn an_actual_that_may_be_null_is_decided_as_before() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    for name in [
        "local_maybe_null",
        "formal_maybe_null",
        "not_structural",
        "non_null_cast",
    ] {
        let (status, leaves) = &rows[name];
        assert_eq!(*status, AnalysisStatus::Unknown, "{name}: {leaves:?}");
        assert!(
            leaves
                .iter()
                .any(|leaf| leaf.contains("retention-undecided")),
            "{name}: {leaves:?}"
        );
    }
    let (status, leaves) = &rows["frame_call_result"];
    assert_ne!(*status, AnalysisStatus::Safe, "{leaves:?}");
}
