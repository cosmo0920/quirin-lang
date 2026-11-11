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

//! Card FB-TEARDOWN-DESTROY-DECLARATIONS (gateway rule 30 at a direct call,
//! contract spec C04.19.6 amended; decision 2 of 2026-10-16, the parameter
//! guard): the positive and negative controls of a destroyer's member-path
//! states read at a direct call on route G, over the synthetic `d_*`
//! callees of `fixtures/contracts/member_path_direct.qv` (trusted test
//! assumptions: a `Safe` here is `Safe` over them and promotes nothing).

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
struct box { struct item *payload; struct item *aux; };
typedef enum { D_KEEP = 0, D_FREE = 1 } d_mode_t;
struct item *d_item_new(int n);
void d_item_free(struct item *it);
struct box *d_box_new(void);
void d_box_free(struct box *b);
void d_box_destroy(struct box *b);
void d_box_destroy_move(struct box *b);
void d_box_destroy_maybe(struct box *b);
void d_box_destroy_del(struct box *b, int del);
void d_box_destroy_mode(struct box *b, d_mode_t mode);
void d_box_drain(struct box *b);
void d_box_destroy_deep(struct box *b);
";

fn environment() -> ResourceContracts {
    let source = include_str!("fixtures/contracts/member_path_direct.qv");
    let source = if cfg!(windows) {
        source.replace("mode: u32", "mode: i32")
    } else {
        source.to_owned()
    };
    load_resource_contract_sources(&BTreeMap::from([(
        "member_path_direct.qv".to_owned(),
        source,
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
            eprintln!("skipping Clang-dependent FB-TEARDOWN-DESTROY-DECLARATIONS test: {reason}");
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

/// The row of `teardown` in the unit `PRELUDE + source`.
fn row(source: &str) -> Row {
    let text = format!("{PRELUDE}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "teardown",
        &environment(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    let row = &summary.functions["teardown"];
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

/// A body that links a fresh item at a fresh box's payload, then runs
/// `teardown` (the calls under test).
fn linking(teardown: &str) -> String {
    format!(
        "void teardown(int n) {{ struct box *b; struct item *it; b = d_box_new(); \
         it = d_item_new(n); if (!it) {{ d_box_free(b); return; }} b->payload = it; \
         {teardown} }}"
    )
}

#[test]
fn a_direct_call_of_a_destroyer_releases_what_its_stated_paths_hold() {
    if !clang_available() {
        return;
    }
    assert!(row(&linking("d_box_destroy(b);")).safe());
    assert!(row(&linking("d_box_destroy_move(b);")).safe());
    // Control: a destroyer that states no path leaves the payload the
    // caller's (premise P-B08-SHALLOW), which then leaks.
    assert!(!row(&linking("d_box_free(b);")).safe());
    // The caller releasing it itself is the control's repair.
    assert!(row(&linking("d_box_free(b); d_item_free(it);")).safe());
}

#[test]
fn a_guard_decided_by_a_constant_selects_its_arm() {
    if !clang_available() {
        return;
    }
    // `del != 0` holds: the payload goes with the box.
    assert!(row(&linking("d_box_destroy_del(b, 1);")).safe());
    // It fails: the payload is kept, so the caller frees it ...
    assert!(row(&linking("d_box_destroy_del(b, 0); d_item_free(it);")).safe());
    // ... and without that it leaks.
    assert!(!row(&linking("d_box_destroy_del(b, 0);")).safe());
    // An enumerator of the parameter's own enumeration, through the alias.
    let mode = row(&linking("d_box_destroy_mode(b, D_FREE);"));
    assert!(mode.safe(), "{mode:?}");
    assert!(row(&linking("d_box_destroy_mode(b, D_KEEP); d_item_free(it);")).safe());
}

#[test]
fn a_guard_on_an_unknown_value_joins_and_refuses() {
    if !clang_available() {
        return;
    }
    let joined = row(&linking("d_box_destroy_del(b, n);"));
    assert!(joined.refused_with("member-path-undecided"), "{joined:?}");
    let joined = row(&linking("d_box_destroy_del(b, n); d_item_free(it);"));
    assert!(joined.refused_with("member-path-undecided"), "{joined:?}");
    // A set refuses the same way.
    let set = row(&linking("d_box_destroy_maybe(b);"));
    assert!(set.refused_with("member-path-undecided"), "{set:?}");
}

#[test]
fn a_double_release_after_the_call_refuses() {
    if !clang_available() {
        return;
    }
    // A stale path: the item is taken and the member still designates it.
    let double = row(&linking("d_box_drain(b); d_item_free(it); d_box_free(b);"));
    assert!(
        double.refused_with("member-path-double-release"),
        "{double:?}"
    );
    // Taken once and never released again: nothing is owed.
    assert!(row(&linking("d_box_drain(b); d_box_free(b);")).safe());
    // A released path released again by the caller is not Safe either.
    assert!(!row(&linking("d_box_destroy(b); d_item_free(it);")).safe());
}

#[test]
fn an_unstated_path_keeps_its_refusal() {
    if !clang_available() {
        return;
    }
    let deep = row(&linking("d_box_destroy_deep(b);"));
    assert!(deep.refused_with("member-path-unstated"), "{deep:?}");
}
