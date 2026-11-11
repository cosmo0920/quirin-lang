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

//! Card OP-CALLBACK-MEMBER-PATH-RELEASE (gateway rule 30, contract spec
//! C04.19.6): the positive and negative controls of a callback candidate's
//! member-path states composed with the destroyer's arms on route G, over
//! the synthetic `c_*` callees of `fixtures/contracts/member_paths.qv`
//! (trusted test assumptions: a `Safe` here is `Safe` over them and promotes
//! nothing).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
#include <stddef.h>
struct item { int x; };
struct op;
typedef int (*op_cb_t)(void *rk, void *rkq, struct op *rko);
struct op { int type; op_cb_t op_cb; struct item *payload; struct item *aux; };
struct item *c_item_new(int n);
struct op *c_op_new_cb(int type, op_cb_t cb);
void c_op_destroy(struct op *rko);
int c_cb_releases(void *rk, void *rkq, struct op *rko);
int c_cb_moves(void *rk, void *rkq, struct op *rko);
int c_cb_stale(void *rk, void *rkq, struct op *rko);
int c_cb_maybe(void *rk, void *rkq, struct op *rko);
int c_cb_deep(void *rk, void *rkq, struct op *rko);
int c_cb_other(void *rk, void *rkq, struct op *rko);
int c_cb_unknown(void *rk, void *rkq, struct op *rko);
";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "member_paths.qv".to_owned(),
        include_str!("fixtures/contracts/member_paths.qv").to_owned(),
    )]))
    .expect("the fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent OP-CALLBACK-MEMBER-PATH-RELEASE test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Row {
    status: AnalysisStatus,
    unsupported: Vec<String>,
}

impl Row {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }
    fn refused_with(&self, key: &str) -> bool {
        self.status != AnalysisStatus::Safe && self.unsupported.iter().any(|d| d.contains(key))
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// The row of `link_and_destroy` in the unit `PRELUDE + source`.
fn row(source: &str) -> Row {
    let text = format!("{PRELUDE}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "link_and_destroy",
        &environment(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    let row = &summary.functions["link_and_destroy"];
    Row {
        status: row.status,
        unsupported: row
            .diagnostics
            .iter()
            .filter_map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect(),
    }
}

/// A body that constructs an op of `type` registering `cb`, links a fresh
/// item at its payload, and destroys the op.
fn linking(type_: u32, cb: &str) -> String {
    format!(
        "void link_and_destroy(int n) {{ struct item *it; struct op *o; it = c_item_new(n); \
         if (!it) {{ return; }} o = c_op_new_cb({type_}, {cb}); o->payload = it; \
         c_op_destroy(o); }}"
    )
}

#[test]
fn a_released_path_then_destroy_is_released_once() {
    if !clang_available() {
        return;
    }
    // Type 1: the arm would release the payload, but the callback released
    // it and cleared the member; type 2: no arm releases it.
    for type_ in [1, 2] {
        let row = row(&linking(type_, "c_cb_releases"));
        assert!(row.safe(), "type {type_}: {row:?}");
    }
}

#[test]
fn a_moved_path_then_destroy_leaves_the_caller() {
    if !clang_available() {
        return;
    }
    for type_ in [1, 2] {
        let row = row(&linking(type_, "c_cb_moves"));
        assert!(row.safe(), "type {type_}: {row:?}");
    }
}

#[test]
fn an_unstated_path_stays_refused() {
    if !clang_available() {
        return;
    }
    // No declaration in force.
    let unknown = row(&linking(1, "c_cb_unknown"));
    assert!(unknown.refused_with("callback-uncontracted"), "{unknown:?}");
    // A declaration stating a path below the linked one, never the payload.
    let deep = row(&linking(1, "c_cb_deep"));
    assert!(deep.refused_with("member-path-unstated"), "{deep:?}");
}

#[test]
fn a_double_release_attempt_is_refused() {
    if !clang_available() {
        return;
    }
    // The callback leaves the payload stale and the type-1 arm releases it.
    let double = row(&linking(1, "c_cb_stale"));
    assert!(
        double.refused_with("member-path-double-release"),
        "{double:?}"
    );
    // No arm releases it: a second invocation would take it again.
    let stale = row(&linking(2, "c_cb_stale"));
    assert!(stale.refused_with("member-path-stale"), "{stale:?}");
}

#[test]
fn a_may_set_is_decided_only_where_an_arm_releases_the_path() {
    if !clang_available() {
        return;
    }
    // Moved, or kept and released by the type-1 arm: gone on both readings.
    let arm = row(&linking(1, "c_cb_maybe"));
    assert!(arm.safe(), "{arm:?}");
    // No arm: moved on one reading, still linked on the other.
    let none = row(&linking(2, "c_cb_maybe"));
    assert!(none.refused_with("member-path-undecided"), "{none:?}");
}

#[test]
fn an_unnamed_path_is_kept_and_left_to_the_arms() {
    if !clang_available() {
        return;
    }
    // `aux` is stated, the payload is not: the type-1 arm releases it.
    let one = row(&linking(1, "c_cb_other"));
    assert!(one.safe(), "{one:?}");
    // Type 2: no arm releases the payload, which the caller loses.
    let two = row(&linking(2, "c_cb_other"));
    assert!(!two.safe(), "{two:?}");
}

/// Card FB-TEARDOWN-DESTROY-DECLARATIONS (rule 30 at a direct call): a
/// direct call of a member-path declaration without a hand-off reads its
/// states once: the payload moved by the direct call is gone before the
/// op's own callback and arms run (before the card the call refused
/// `member-path-direct`, which a declaration beside a hand-off still does).
#[test]
fn a_direct_call_of_a_member_path_declaration_reads_its_states() {
    if !clang_available() {
        return;
    }
    let direct = row(
        "void link_and_destroy(int n) { struct item *it; struct op *o; it = c_item_new(n); \
         if (!it) { return; } o = c_op_new_cb(1, c_cb_moves); o->payload = it; \
         c_cb_moves(NULL, NULL, o); c_op_destroy(o); }",
    );
    assert!(direct.safe(), "{direct:?}");
    // A stale direct call: the type-1 arm then releases the payload again.
    let stale = row(
        "void link_and_destroy(int n) { struct item *it; struct op *o; it = c_item_new(n); \
         if (!it) { return; } o = c_op_new_cb(1, c_cb_moves); o->payload = it; \
         c_cb_stale(NULL, NULL, o); c_op_destroy(o); }",
    );
    assert!(!stale.safe(), "{stale:?}");
}

#[test]
fn a_body_that_contradicts_its_states_withdraws_them_in_its_unit() {
    if !clang_available() {
        return;
    }
    // The unit defines the candidate: a null store into the stated path
    // before every return validates it; the composition stands.
    let validated = row(&format!(
        "int c_cb_moves(void *rk, void *rkq, struct op *rko) {{ rko->payload = NULL; return 0; }}\n{}",
        linking(2, "c_cb_moves")
    ));
    assert!(validated.safe(), "{validated:?}");
    // A pointer store into a path the declaration does not state: withdrawn
    // here, so the candidate has no contract in force.
    let withdrawn = row(&format!(
        "int c_cb_moves(void *rk, void *rkq, struct op *rko) {{ rko->payload = NULL; \
         rko->aux = NULL; return 0; }}\n{}",
        linking(2, "c_cb_moves")
    ));
    assert!(
        withdrawn.refused_with("callback-uncontracted"),
        "{withdrawn:?}"
    );
}
