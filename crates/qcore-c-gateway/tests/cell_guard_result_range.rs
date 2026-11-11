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

//! Card CELL-GUARD-RESULT-RANGE-CHECKED (gateway rule 38e, contract
//! specification C09.4; USER DECISION 2026-11-03, option C): rule 38 reads a
//! caller's comparison of a channel creator's status witness through the
//! creator's pin-derived result set, and only while that set is in force.
//!
//! The declarations are the provider's own restated Monkey declarations
//! (`providers/qcore-provider-libevent/contracts/monkey/event_loop.qv`) and
//! the synthetic allocator of `fixtures/contracts/event_loop.qv`. The derived
//! set {-1, 0} is installed as an audit route installs it (the resource
//! environment's result-range table); its standing is the fixture's. A `Safe`
//! here is `Safe` over those declared assumptions and the stated set, and
//! promotes nothing. Every test asserts the complete diagnostic vector of the
//! creating function.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::result_range::{ResultRangeFact, ResultRangeStanding};

const PRELUDE: &str = "\
struct mk_event_loop;
struct mk_event { int fd; int mask; };
struct chan { int r; int w; struct mk_event event; };
struct mk_event_loop *mk_event_loop_create(int size);
void mk_event_loop_destroy(struct mk_event_loop *loop);
int mk_event_channel_create(struct mk_event_loop *loop, int *r_fd, int *w_fd, void *data);
int mk_event_channel_destroy(struct mk_event_loop *loop, int r_fd, int w_fd, void *data);
";

const INT: (i64, i64) = (i32::MIN as i64, i32::MAX as i64);

fn environment(standing: Option<ResultRangeStanding>) -> ResourceContracts {
    let mut contracts = load_resource_contract_sources(&BTreeMap::from([
        (
            "event_loop.qv".to_owned(),
            include_str!("fixtures/contracts/event_loop.qv").to_owned(),
        ),
        (
            "monkey/event_loop.qv".to_owned(),
            include_str!(
                "../../../providers/qcore-provider-libevent/contracts/monkey/event_loop.qv"
            )
            .to_owned(),
        ),
    ]))
    .expect("the contracts load");
    if let Some(standing) = standing {
        contracts.set_result_ranges(BTreeMap::from([(
            "mk_event_channel_create".to_owned(),
            ResultRangeFact {
                intervals: vec![(-1, 0)],
                universe: INT,
                standing,
                detail: "fixture".to_owned(),
            },
        )]));
    }
    contracts
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CELL-GUARD-RESULT-RANGE-CHECKED test: {reason}");
            false
        }
    }
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("no projection at all: {other:?}"),
    }
}

/// The status and the complete, sorted diagnostic vector of `cycle`, whose
/// channel test is `test` (written over `ret`).
fn outcome(test: &str, standing: Option<ResultRangeStanding>) -> (&'static str, Vec<String>) {
    let source = format!(
        "{PRELUDE}
int cycle(struct chan *c) {{
    struct mk_event_loop *loop = mk_event_loop_create(8);
    if (!loop) {{ return -1; }}
    int ret = mk_event_channel_create(loop, &c->r, &c->w, &c->event);
    if ({test}) {{ mk_event_loop_destroy(loop); return -1; }}
    mk_event_channel_destroy(loop, c->r, c->w, &c->event);
    mk_event_loop_destroy(loop);
    return 0;
}}
"
    );
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "cycle",
        &environment(standing),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    let status = match &result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate)
                .expect("a Safe result carries a certificate that rechecks");
            "Safe"
        }
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    };
    let mut keys = program(&result)
        .functions
        .get("cycle")
        .expect("missing function `cycle`")
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported@{}:{description}", diagnostic.line)
            }
            other => format!("{other:?}@{}", diagnostic.line),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

/// What every undecided reading leaves, diagnostic for diagnostic: the
/// creation is never decided, so the destroy's guard is undecided and every
/// cell may outlive the frame at both exits.
const UNDECIDED: [&str; 7] = [
    "unsupported@12:the descriptor in cell `(c as int).r [Name(\"c\") as int]` may outlive this function's frame (descriptor-cell-escapes)",
    "unsupported@12:the descriptor in cell `(c as int).w [Name(\"c\") as int]` may outlive this function's frame (descriptor-cell-escapes)",
    "unsupported@12:the descriptor in cell `(c.event as mk_event).fd [Name(\"c.event\") as mk_event]` may outlive this function's frame (descriptor-cell-escapes)",
    "unsupported@13:call to `mk_event_channel_destroy`: descriptor cell `(Interior(\"c\", [\"event\"], true) as mk_event).fd` is not decided (descriptor-cell-guard-undecided)",
    "unsupported@15:the descriptor in cell `(c as int).r [Name(\"c\") as int]` may outlive this function's frame (descriptor-cell-escapes)",
    "unsupported@15:the descriptor in cell `(c as int).w [Name(\"c\") as int]` may outlive this function's frame (descriptor-cell-escapes)",
    "unsupported@15:the descriptor in cell `(c.event as mk_event).fd [Name(\"c.event\") as mk_event]` may outlive this function's frame (descriptor-cell-escapes)",
];

fn undecided() -> (&'static str, Vec<String>) {
    (
        "Unknown",
        UNDECIDED.iter().map(|key| (*key).to_owned()).collect(),
    )
}

/// Both caller spellings at a channel creator. `!= 0` is exact on every
/// `int` and decides with or without a set; `== -1` (Fluent Bit's) decides
/// only under the set in force, where it equals the contract's split.
#[test]
fn both_caller_spellings_decide_under_the_set_in_force() {
    if !clang_available() {
        return;
    }
    let in_force = Some(ResultRangeStanding::InForce);
    assert_eq!(outcome("ret != 0", None), ("Safe", Vec::new()));
    assert_eq!(outcome("ret != 0", in_force), ("Safe", Vec::new()));
    assert_eq!(outcome("ret == -1", in_force), ("Safe", Vec::new()));
    assert_eq!(outcome("-1 == ret", in_force), ("Safe", Vec::new()));
    assert_eq!(outcome("ret < 0", in_force), ("Safe", Vec::new()));
    assert_eq!(outcome("ret <= -1", in_force), ("Safe", Vec::new()));
}

/// Out of force, the set is never read: an unchecked range (pinned, never
/// derived), a held one, or none at all leave `== -1` exactly as undecided
/// as before the card, diagnostic for diagnostic.
#[test]
fn a_range_out_of_force_changes_nothing() {
    if !clang_available() {
        return;
    }
    assert_eq!(outcome("ret == -1", None), undecided());
    assert_eq!(outcome("ret < 0", None), undecided());
    for standing in [ResultRangeStanding::Unchecked, ResultRangeStanding::Held] {
        assert_eq!(
            outcome("ret == -1", Some(standing)),
            undecided(),
            "{standing:?}"
        );
        assert_eq!(
            outcome("ret < 0", Some(standing)),
            undecided(),
            "{standing:?}"
        );
    }
}

/// A refuted range is never in force: a derivation the body refutes is
/// never pinned, and one whose C library sets changed is held
/// (`contract-changed`), which reads exactly as no range.
#[test]
fn a_refuted_range_changes_nothing() {
    if !clang_available() {
        return;
    }
    let mut contracts = environment(None);
    contracts.set_result_ranges(BTreeMap::from([(
        "mk_event_channel_create".to_owned(),
        ResultRangeFact {
            intervals: vec![(-1, 0)],
            universe: INT,
            standing: ResultRangeStanding::Held,
            detail: "contract-changed: the C library result set of `pipe` the derivation rested on changed".to_owned(),
        },
    )]));
    assert!(
        contracts
            .result_range_in_force("mk_event_channel_create")
            .is_none()
    );
    assert_eq!(
        outcome("ret == -1", Some(ResultRangeStanding::Held)),
        undecided()
    );
}

/// A test against a constant the set excludes, or one that misclassifies a
/// value of the set, stays undecided: never smoothed over.
#[test]
fn an_out_of_range_or_wrong_test_stays_undecided() {
    if !clang_available() {
        return;
    }
    let in_force = Some(ResultRangeStanding::InForce);
    // A constant the set excludes (the body never returns -2).
    assert_eq!(outcome("ret == -2", in_force), undecided());
    assert_eq!(outcome("ret != -2", in_force), undecided());
    // A test that reads 0, a success, as failure (`> 0`), or -1, a failure,
    // as success (`>= -1`).
    assert_eq!(outcome("ret > 0", in_force), undecided());
    assert_eq!(outcome("ret >= -1", in_force), undecided());
}
