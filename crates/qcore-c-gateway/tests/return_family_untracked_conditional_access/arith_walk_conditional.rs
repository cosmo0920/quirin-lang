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

//! Card **RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL** (gateway rule 82):
//! family S of W45-RETURN-EXPRESSION-FAMILIES before an accessed
//! conditional whose arm is a sum with the root as a later term, end to end
//! through the real gateway.
//!
//! At the base rule 74w's walk read the conditional's parentheses as the
//! additive group around `p` in `return (k ? 1 + p - k : q)->n;` and
//! refused, so the expression fell to the later routes. The walk now stops at
//! the arm, rule 74x reads it, and both arms' roots are bases. Every row
//! asserts the status and the complete set of diagnostic keys.
//!
//! These rows are **not** change pins: each one reads the same with the new
//! reading disabled, because at the base rule 10z's connective return lowered
//! the conditional family S refused by branching and read each arm (report
//! §3). They pin that the relaxation keeps the end-to-end verdicts; the
//! change itself is pinned at unit level
//! (`tests/unit/legacy_return_family_arith_walk_conditional.rs`).
//!
//! The fixtures use the generic `k_*` surface of
//! `fixtures/contracts/descriptor_generic.qcore`; `Safe` here is `Safe` over
//! those synthetic trusted test assumptions.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *k_alloc(__SIZE_TYPE__ size); \
void k_free(void *p); \
struct s { long n; long a[2]; struct s *next; };";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "descriptor_generic.qcore".to_owned(),
        include_str!("../fixtures/contracts/descriptor_generic.qv").to_owned(),
    )]))
    .expect("the fixtures load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent return-family test: {reason}");
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

fn key(kind: &LegacyDiagnosticKind) -> String {
    match kind {
        LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
        LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
        LegacyDiagnosticKind::Unsupported { description } => format!("unsupported:{description}"),
        other => format!("{other:?}")
            .split([' ', '{'])
            .next()
            .unwrap_or("")
            .to_owned(),
    }
}

/// The status of `w` and the sorted, de-duplicated keys of its diagnostics.
fn outcome(body: &str) -> (&'static str, Vec<String>) {
    let source = format!("{DECLARATIONS} {body}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
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
    let function = program(&result).functions.get("w").expect("w analysed");
    if status == "Safe" {
        assert_eq!(function.status, AnalysisStatus::Safe);
    }
    let mut keys = function
        .diagnostics
        .iter()
        .map(|diagnostic| key(&diagnostic.kind))
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

fn expect(label: &str, body: &str, status: &str, vector: &[&str]) {
    let (measured, keys) = outcome(body);
    assert_eq!(
        (
            measured,
            keys.iter().map(String::as_str).collect::<Vec<_>>()
        ),
        (status, vector.to_vec()),
        "{label}\n{body}"
    );
}

const BOTH: &[&str] = &["unknown_alias:p", "unknown_alias:q"];

/// An accessed conditional whose arm is a sum with the null untracked `p` as
/// a later term: both arms' null roots are bases, so the function is
/// `Unknown` with both `unknown_alias` keys.
#[test]
fn null_roots_of_an_arm_sum_are_bases() {
    if !clang_available() {
        return;
    }
    for (label, pointer, expression, keys) in [
        (
            "middle arm, member",
            "struct s",
            "(k ? 1 + p - k : q)->n",
            BOTH,
        ),
        ("last arm, element", "long", "(k ? q : k + p)[0]", BOTH),
        (
            "last arm, dereference",
            "long",
            "*(k ? q : k + p - 1)",
            BOTH,
        ),
        (
            "sum around",
            "struct s",
            "((k ? 1 + p - k : q) + 1)->n",
            BOTH,
        ),
        (
            "nested, last arm",
            "struct s",
            "(k ? q : k > 1 ? r : 1 + p - k)->n",
            &["unknown_alias:p", "unknown_alias:q", "unknown_alias:r"],
        ),
    ] {
        expect(
            label,
            &format!(
                "long w(long k) {{ const {pointer} *p = 0; const {pointer} *q = 0; \
                 const {pointer} *r = 0; return {expression}; }}"
            ),
            "Unknown",
            keys,
        );
    }
}

/// Unbound formals in both arms: their bases discharge, so the function is
/// `Safe`.
#[test]
fn unbound_formals_in_both_arms_are_safe() {
    if !clang_available() {
        return;
    }
    for expression in [
        "(k ? 1 + p - k : q)->n",
        "(k ? q : k + p)[0].n",
        "((k ? 1 + p - k : q) + 1)->n",
    ] {
        expect(
            "unbound formals",
            &format!(
                "long w(long k, const struct s *p, const struct s *q) {{ return {expression}; }}"
            ),
            "Safe",
            &[],
        );
    }
}

/// What still refuses family S reads as its later routes do: a root only in
/// the condition, a may-bind name in an arm, a nested conditional inside a
/// sum, and `?:` inside a call's argument.
#[test]
fn the_other_refusals_stand() {
    if !clang_available() {
        return;
    }
    let three = "const struct s *p = 0; const struct s *q = 0; const struct s *r = 0;";
    for (label, body, status, keys) in [
        (
            "root only in the condition",
            format!("long w(long k) {{ {three} return (1 + p - k ? q : r)->n; }}"),
            "Unknown",
            &["unknown_alias:q", "unknown_alias:r"] as &[&str],
        ),
        (
            "may-bind name in an arm",
            "long w(long k) { const struct s *p = 0; struct s *m = k_alloc(sizeof *m); \
             if (!m) return 0; return (k ? 1 + p - k : m)->n; }"
                .to_owned(),
            "Unknown",
            REFUSED,
        ),
        (
            "nested conditional inside a sum",
            format!(
                "long w(long k) {{ {three} return (k ? 1 + (k ? 1 + p - k : q) - k : r)->n; }}"
            ),
            "Unknown",
            &["unknown_alias:p", "unknown_alias:q", "unknown_alias:r"],
        ),
        (
            "call argument",
            "const struct s *g(const struct s *x); \
             long w(long k) { const struct s *p = 0; const struct s *q = 0; \
             return g(k ? 1 + p - k : q)->n; }"
                .to_owned(),
            "Unknown",
            REFUSED,
        ),
    ] {
        expect(label, &body, status, keys);
    }
}

/// Family S refuses, and so does every older route: the return refusal.
const REFUSED: &[&str] =
    &["unsupported:unmodeled call, aggregate or indirect access in return expression"];
