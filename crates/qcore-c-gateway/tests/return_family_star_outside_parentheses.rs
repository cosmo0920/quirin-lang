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

//! Card **RETURN-FAMILY-STAR-OUTSIDE-ROOT-PARENTHESES** (gateway rule 74s):
//! family S of W45-RETURN-EXPRESSION-FAMILIES under a `*` that sits outside a
//! starred may-bind root's own parentheses, end to end through the real
//! gateway.
//!
//! At the base `stars_before` stopped a starred root's count at a `*` behind
//! more opening parentheses than close right after the root, or at the binary
//! operator before the root, so `return *(*q + 1);` counted one dereference of
//! `q`, family S admitted it, and the outer dereference of the pointer
//! computed from `*q` carried no obligation: the function was `Safe`, while
//! `return **q;` keeps the return refusal. Every row asserts the status and
//! the complete set of diagnostic keys.
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
void k_free(void *p);";

const RETURN: &str =
    "unsupported:unmodeled call, aggregate or indirect access in return expression";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "descriptor_generic.qcore".to_owned(),
        include_str!("fixtures/contracts/descriptor_generic.qv").to_owned(),
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

/// A `*` (or a subscript or `->`) applied to a parenthesised expression that
/// contains `*q`, with `q` a may-bind formal: two dereferences, refused by
/// family S as `return **q;` is. At the base every row but the control was
/// `Safe`.
#[test]
fn a_star_outside_the_root_parentheses_is_a_second_dereference() {
    if !clang_available() {
        return;
    }
    for (label, body) in [
        (
            "pointer arithmetic",
            "long w(long **q) { return *(*q + 1); }",
        ),
        (
            "cast inside the group",
            "long w(long **q) { return *((long *)*q + 1); }",
        ),
        ("root second", "long w(long **q) { return *(1 + *q); }"),
        ("subtraction", "long w(long **q) { return *(*q - 1); }"),
        (
            "cast outside the group",
            "long w(long **q) { return *(long *)(*q + 1); }",
        ),
        (
            "subscript of the group",
            "long w(long **q) { return (*q + 1)[0]; }",
        ),
        (
            "no arithmetic (control)",
            "long w(long **q) { return **q; }",
        ),
    ] {
        expect(label, body, "Unknown", &[RETURN]);
    }
}

/// The same after `k_free(q)`. At the base family S admitted `q` as one
/// dereference base and the flow check accused the first dereference
/// (`Unsafe`, `UseAfterRelease`); the tip refuses the expression as it refuses
/// `**q`.
#[test]
fn a_released_root_under_an_outside_star_is_refused() {
    if !clang_available() {
        return;
    }
    for (label, expression) in [
        ("pointer arithmetic", "*(*q + 1)"),
        ("cast inside the group", "*((long *)*q + 1)"),
        ("no arithmetic (control)", "**q"),
    ] {
        expect(
            label,
            &format!(
                "long w(void) {{ long **q = k_alloc(8); if (!q) return 0; k_free(q); \
                 return {expression}; }}"
            ),
            "Unknown",
            &[RETURN],
        );
    }
}

/// Controls the card keeps at their base readings: the bare dereference, its
/// parenthesised spelling, an arithmetic read with no outer dereference and a
/// cast of one dereference.
#[test]
fn one_dereference_and_an_arithmetic_read_are_unchanged() {
    if !clang_available() {
        return;
    }
    for (label, expression) in [
        ("bare dereference", "*q"),
        ("parenthesised", "(*q)"),
        ("arithmetic read", "*q + 1"),
        ("cast of one dereference", "(long) *q"),
    ] {
        expect(
            label,
            &format!("long w(long *q) {{ return {expression}; }}"),
            "Safe",
            &[],
        );
        expect(
            label,
            &format!(
                "long w(void) {{ long *q = k_alloc(8); if (!q) return 0; k_free(q); \
                 return {expression}; }}"
            ),
            "Unsafe",
            &["UseAfterRelease", "unknown_alias:q"],
        );
    }
    // Rule 74m's refusal is kept.
    expect(
        "cast of a dereference, dereferenced",
        "long w(long **q) { return *(long *)*q; }",
        "Unknown",
        &[RETURN],
    );
}
