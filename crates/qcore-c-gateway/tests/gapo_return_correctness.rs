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

//! Card GAPO-RETURN-CORRECTNESS (gateway rules 15x, 15y, 15z): route-G
//! controls.
//!
//! The harness is CG00's (`support/clang_gap.rs`) plus, for rule 15y, the
//! production `quirin-provider-libc` `<stdlib.h>` module byte for byte (`exit`
//! with `resource_no_return()`), as card CG21's controls load it. A `Safe`
//! here is `Safe` over those trusted assumptions.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyInstruction, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/clang_gap.rs"]
mod clang_gap;

use clang_gap::{DECLARATIONS, clang_available, contract_sources};

const STDLIB: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/stdlib.qv");

const RECORDS: &str = "struct T { int x; char *name; };\nstruct R { int code; };\n";

const FIXTURE: &str = include_str!("fixtures/contracts/gapo_return_correctness.qv");

fn contracts() -> ResourceContracts {
    let mut sources: BTreeMap<String, String> = contract_sources();
    sources.insert("libc/stdlib.qv".to_owned(), STDLIB.to_owned());
    sources.insert(
        "gapo_return_correctness.qcore".to_owned(),
        FIXTURE.to_owned(),
    );
    load_resource_contract_sources(&sources).expect("the fixture contracts load")
}

fn run(prelude: &str, source: &str) -> LegacyFunctionSummary {
    let text = format!("{DECLARATIONS}\n{RECORDS}{prelude}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    summary
        .and_then(|summary| summary.functions.get("subject"))
        .unwrap_or_else(|| panic!("no summary for `subject`: {outcome:?}"))
        .clone()
}

fn unsupported(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .filter_map(|d| match &d.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect()
}

/// Every site a node mints, in node order.
fn sites(summary: &LegacyFunctionSummary) -> Vec<usize> {
    summary
        .cfg
        .nodes
        .iter()
        .flat_map(|node| match &node.instruction {
            LegacyInstruction::Acquire { site, .. } => vec![*site],
            LegacyInstruction::ContractCall { produced_sites, .. } => produced_sites.clone(),
            _ => Vec::new(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rule 15x: a replacing classification leaves no gap.
// ---------------------------------------------------------------------------

/// Rule 12h classifies `u = r_wrap((void *) t)` again as `u = r_wrap(t)`.
/// BEFORE (main `1a399e6a1`) the first classification's site stayed minted,
/// so the two sites were 1 and 2 (0 minted and dropped); with rule 15x the
/// second classification starts from the first's state and they are 0 and
/// 1.
#[test]
fn a_replacing_classification_leaves_no_site_gap() {
    if !clang_available() {
        return;
    }
    let summary = run(
        "",
        "void subject(struct T *t) { void *u; u = r_wrap((void *) t); \
         void *v = q_alloc(); q_free(v); q_free(u); }",
    );
    assert_eq!(summary.status, AnalysisStatus::Safe, "{summary:?}");
    let mut minted = sites(&summary);
    minted.sort_unstable();
    minted.dedup();
    assert_eq!(minted, (0..minted.len()).collect::<Vec<_>>(), "{summary:?}");
}

// ---------------------------------------------------------------------------
// Rule 15y: a no-return call ends its path at every lowered position.
// ---------------------------------------------------------------------------

const NO_RETURN: &str = "__attribute__((noreturn)) void gr_fatalf(const char *format, ...);\n\
                         __attribute__((noreturn)) void gr_die(int value);";
const RETURNS: &str = "void gr_fatalf(const char *format, ...);\nvoid gr_die(int value);";

/// `if (n > 0) { q_free(q); CALL } q_free(q);` over the declared no-return
/// call `CALL`: a double release exactly when the path after `CALL`
/// continues.
fn after_release(statement: &str) -> String {
    format!(
        "void subject(struct R *r, struct T *t, int n) {{ void *q = q_alloc(); \
         if (n > 0) {{ q_free(q); {statement} }} q_free(q); }}"
    )
}

/// The node of a call to `callee`, if one is in the CFG.
fn call_node<'s>(
    summary: &'s LegacyFunctionSummary,
    callee: &str,
) -> Option<&'s qcore_c_gateway::legacy_c::LegacyCfgNode> {
    summary.cfg.nodes.iter().find(|node| {
        matches!(&node.instruction,
            LegacyInstruction::ContractCall { function, .. } if function == callee)
    })
}

/// Two calls rule 11o's statement hook never saw, because a later route
/// lowered them: `gr_die(t->x + n)` (the lexical gate's refusal, lowered by
/// W44 as the member read and then the call) and `gr_fatalf("%p", (void *)
/// t)` (rule 12h's H-U, which lowers the uncast statement through rule 11w's
/// tail widening). BEFORE (main `1a399e6a1`) each node kept its
/// continuation, so the second release of `q` was a double release
/// (`Unsafe`); with rule 15y each call ends its path.
#[test]
fn a_no_return_call_lowered_later_ends_its_path() {
    if !clang_available() {
        return;
    }
    for (statement, callee) in [
        ("gr_die(t->x + n);", "gr_die"),
        ("gr_fatalf(\"%p\", (void *) t);", "gr_fatalf"),
    ] {
        let summary = run(NO_RETURN, &after_release(statement));
        assert_eq!(
            summary.status,
            AnalysisStatus::Safe,
            "{statement}: {summary:?}"
        );
        let node = call_node(&summary, callee).expect("the call is lowered");
        assert!(
            node.successors.is_empty() && !node.is_exit,
            "{statement}: {node:?}"
        );
    }
}

/// The same calls to callees whose Clang declaration is not `noreturn` are
/// refused: the declaration alone never ends a path, and the call never
/// continues either.
#[test]
fn an_unestablished_no_return_call_lowered_later_is_refused() {
    if !clang_available() {
        return;
    }
    for (statement, callee) in [
        ("gr_die(t->x + n);", "gr_die"),
        ("gr_fatalf(\"%p\", (void *) t);", "gr_fatalf"),
    ] {
        let summary = run(RETURNS, &after_release(statement));
        assert_eq!(
            summary.status,
            AnalysisStatus::Unknown,
            "{statement}: {summary:?}"
        );
        assert!(
            call_node(&summary, callee).is_none(),
            "{statement}: {summary:?}"
        );
        assert!(
            unsupported(&summary).iter().any(|description| {
                description.contains(&format!("no-return contract of `{callee}`"))
            }),
            "{statement}: {summary:?}"
        );
    }
}

/// Rule 11o's own statement position is unchanged: the hook's path end is
/// a node with no successor, which the settlement leaves alone.
#[test]
fn the_statement_hook_path_end_is_unchanged() {
    if !clang_available() {
        return;
    }
    let summary = run(NO_RETURN, &after_release("gr_die(n);"));
    assert_eq!(summary.status, AnalysisStatus::Safe, "{summary:?}");
    let node = call_node(&summary, "gr_die").expect("the call is lowered");
    assert!(node.successors.is_empty() && !node.is_exit, "{node:?}");
}

// ---------------------------------------------------------------------------
// Rule 15z: rule 12h's uncast actuals at the return position.
// ---------------------------------------------------------------------------

/// W4's whole returned call, B01's scalar call and W45-K's converted call
/// read the cast actual `(void *) t` as no place, so each use after release
/// was `Safe` BEFORE (main `1a399e6a1`, checked with the hook disabled); the
/// uncast call reads `t`.
#[test]
fn a_cast_actual_of_a_returned_call_is_its_place() {
    if !clang_available() {
        return;
    }
    for body in [
        "int subject(void) { struct T *t = q_alloc(); q_free((void *) t); \
         return r_borrow_code((void *) t); }",
        // B01's scalar call.
        "int subject(void) { struct T *t = q_alloc(); q_free((void *) t); \
         return 1 + r_borrow_code((void *) t); }",
        // W45-K's converted call.
        "long subject(void) { struct T *t = q_alloc(); q_free((void *) t); \
         return (long) r_borrow_code((void *) t); }",
    ] {
        let summary = run("", body);
        assert_ne!(summary.status, AnalysisStatus::Safe, "{body}: {summary:?}");
    }
}

/// Controls: the same reads of a live or caller-held place stay `Safe`.
#[test]
fn a_cast_actual_of_a_live_place_stays_safe() {
    if !clang_available() {
        return;
    }
    for body in [
        "int subject(struct T *t) { return r_borrow_code((void *) t); }",
        "int subject(struct T *t) { return 1 + r_borrow_code((void *) t); }",
        "int subject(void) { struct T *t = q_alloc(); int r = r_borrow_code((void *) t); \
         q_free(t); return r; }",
    ] {
        let summary = run("", body);
        assert_eq!(summary.status, AnalysisStatus::Safe, "{body}: {summary:?}");
    }
}
