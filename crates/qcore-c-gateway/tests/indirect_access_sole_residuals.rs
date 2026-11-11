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

//! Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (gateway rule 12k).
//!
//! The gate's refusal of a statement whose call's actuals read memory, when
//! the callee has no contract in force, names the uncontracted callee where
//! the callee is what decides: with every memory-reading actual read as at a
//! `Borrowed` position the statement is refused for exactly that call. The
//! node stays the same refusal, so the status never moves. Every attribution
//! has a control that keeps the gate's refusal byte for byte.
//!
//! The fixture contracts are the CG00 harness's `q_*` assumptions and
//! `indirect_sole.qv` (card CG10-INDIRECT-ACCESS-SOLE-BLOCKERS). Trusted test
//! assumptions, not library facts.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p); void q_observe(void *p);
int is_cmp(const char *a, const char *b); char *is_dup(const char *p);
int is_count(const char *p); int is_unknown(const char *p);
int is_unknown2(const char *p, int n);
struct S { int a; char *name; struct S *next; };
";

const INDIRECT: &str = "indirect memory access is outside the resource projection";

fn contracts() -> ResourceContracts {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    load_resource_contract_sources(&BTreeMap::from([
        file("q_base.qcore", include_str!("fixtures/contracts/q_base.qv")),
        file(
            "q_rich_modes.qcore",
            include_str!("fixtures/contracts/q_rich_modes.qv"),
        ),
        file(
            "indirect_sole.qcore",
            include_str!("fixtures/contracts/indirect_sole.qv"),
        ),
    ]))
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

fn run(source: &str) -> Run {
    let text = format!("{DECLARATIONS}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    let result = summary
        .and_then(|summary| summary.functions.get("subject"))
        .unwrap_or_else(|| panic!("`subject` projected: {outcome:?}"));
    Run {
        status: result.status,
        descriptions: result
            .diagnostics
            .iter()
            .filter_map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect(),
    }
}

/// The function stays `Unknown` and its one refusal names `callee`.
fn assert_names_the_callee(source: &str, callee: &str) {
    let run = run(source);
    assert_eq!(run.status, AnalysisStatus::Unknown, "{source}\n{run:?}");
    assert_eq!(
        run.descriptions,
        vec![format!("call to uncontracted function `{callee}`")],
        "{source}\n{run:?}"
    );
}

/// The function stays non-`Safe` and keeps the gate's refusal.
fn assert_keeps_the_gate(source: &str) {
    let run = run(source);
    assert_ne!(run.status, AnalysisStatus::Safe, "{source}\n{run:?}");
    assert!(
        run.descriptions
            .iter()
            .any(|description| description == INDIRECT),
        "{source}\n{run:?}"
    );
}

#[test]
fn a_call_statement_names_its_uncontracted_callee() {
    if !clang_available() {
        return;
    }
    assert_names_the_callee(
        "void subject(struct S *a) { is_unknown(a->name); }",
        "is_unknown",
    );
    assert_names_the_callee(
        "int subject(struct S *a) { int n = is_unknown2(a->name, a->a + 1); return n; }",
        "is_unknown2",
    );
}

#[test]
fn a_whole_call_return_names_its_uncontracted_callee() {
    if !clang_available() {
        return;
    }
    assert_names_the_callee(
        "int subject(struct S *a) { return is_unknown(a->name); }",
        "is_unknown",
    );
}

#[test]
fn the_same_call_with_a_contract_is_read_by_rule_w44() {
    if !clang_available() {
        return;
    }
    // The control that the callee is what decides: `is_count` is declared
    // `Borrowed`, and the same statement is admitted.
    let run = run("void subject(struct S *a) { is_count(a->name); }");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

#[test]
fn an_actual_neither_reader_recognises_keeps_the_gate_refusal() {
    if !clang_available() {
        return;
    }
    // A call inside the actual: the recogniser refuses it whatever the
    // callee's contract.
    assert_keeps_the_gate(
        "void subject(struct S *a) { is_unknown2(a->name, is_count(a->next->name)); }",
    );
}

#[test]
fn an_access_outside_the_actuals_keeps_the_gate_refusal() {
    if !clang_available() {
        return;
    }
    // The destination is a member store: reading the actuals does not reach
    // the call, the gate refuses the destination.
    assert_keeps_the_gate("void subject(struct S *a) { a->next->a = is_unknown(a->name); }");
}

#[test]
fn a_contracted_callee_at_a_custody_position_keeps_the_gate_refusal() {
    if !clang_available() {
        return;
    }
    // `q_free` is contracted: W44 decides, rule 12k is not asked.
    // Card MEMBER-EFFECT-SUMMARY (rule 38i) re-pins this deliberately: the non-null release of a formal's member
    // names its premise; still `Unknown`.
    let source = "void subject(struct S *a) { q_free(a->name); }";
    let run = run(source);
    assert_eq!(run.status, AnalysisStatus::Unknown, "{source}\n{run:?}");
    assert_eq!(
        run.descriptions,
        vec![
            "release of `a->name`, a member of the formal `a` (member-release-through-formal)"
                .to_owned()
        ],
        "{source}\n{run:?}"
    );
}
