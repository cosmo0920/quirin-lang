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

//! Card **RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST** (gateway rule 74r):
//! family S of W45-RETURN-EXPRESSION-FAMILIES before an untracked root that
//! a `*` dereferences through a cast group, end to end through the real
//! gateway.
//!
//! At the base an untracked root was a starred base of the read node only
//! when the token right before it was a `*`, so `return *(long *)p;` through
//! a null `p` made no base, the flow check never saw the dereference, and the
//! function was `Safe`, while `return *p;` is `Unknown` (`unknown_alias:p`).
//! Every row asserts the status and the complete set of diagnostic keys.
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

/// `return *(T *)p;` through a null untracked `p`: the dereference makes `p`
/// a base of the read node, as `return *p;` does, through one cast group,
/// nested groups and parentheses. At the base every row was `Safe`.
#[test]
fn a_null_untracked_root_through_a_cast_is_a_base() {
    if !clang_available() {
        return;
    }
    for (label, expression) in [
        ("one cast", "*(long *)p"),
        ("qualified cast", "*(const long *)p"),
        ("nested casts", "*(long *)(const long *)p"),
        ("byte view and back", "*(long *)(char *)p"),
        ("parenthesised cast", "*((long *)p)"),
        ("parenthesised root", "*(long *)(p)"),
        ("in a sum", "*(long *)p + 1"),
        ("cast of the dereference", "(int) *(long *)p"),
        ("parenthesised root, no cast", "*(p)"),
        ("pointer arithmetic in parentheses", "*(p + 1)"),
    ] {
        expect(
            label,
            &format!("long w(void) {{ const long *p = 0; return {expression}; }}"),
            "Unknown",
            &["unknown_alias:p"],
        );
    }
}

/// The readings the card keeps: the bare and cast dereferences of the null
/// root (bases already), an unbound formal, whose base obligation discharges
/// through a cast as without one, and a tracked root.
#[test]
fn the_base_readings_are_kept() {
    if !clang_available() {
        return;
    }
    for (label, expression) in [
        ("bare dereference", "*p"),
        ("cast of one dereference", "(long) *p"),
    ] {
        expect(
            label,
            &format!("long w(void) {{ const long *p = 0; return {expression}; }}"),
            "Unknown",
            &["unknown_alias:p"],
        );
    }
    for expression in ["*p", "*(long *)p", "*(long *)(char *)p"] {
        expect(
            "unbound formal",
            &format!("long w(const long *p) {{ return {expression}; }}"),
            "Safe",
            &[],
        );
    }
    // A tracked `p` is a may-bind root, which this card leaves as it was:
    // the same verdict through a cast as without one.
    for expression in ["*p", "*(long *)p"] {
        expect(
            "tracked non-null root",
            &format!(
                "long w(void) {{ long *p = k_alloc(8); if (!p) return 0; return {expression}; }}"
            ),
            "Unsafe",
            &["leak"],
        );
        expect(
            "tracked released root",
            &format!(
                "long w(void) {{ long *p = k_alloc(8); if (!p) return 0; k_free(p); \
                 return {expression}; }}"
            ),
            "Unsafe",
            &["UseAfterRelease", "unknown_alias:p"],
        );
    }
}
