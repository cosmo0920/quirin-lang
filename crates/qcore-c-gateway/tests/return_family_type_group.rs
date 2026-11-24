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

//! Card **TYPE-GROUP-QUALIFIED-POINTER** (gateway rule 74u): family S of
//! W45-RETURN-EXPRESSION-FAMILIES before an untracked root that a `*`
//! dereferences through a cast group whose pointer declarator carries a
//! qualifier after its `*`, end to end through the real gateway.
//!
//! At the base `cast_semantics::type_group_open` read no such group, so rule
//! 74r's reading stopped at it: `return *(long * const)p;` through a null
//! `p` made no base and the function was `Safe`, while `return *(long *)p;`
//! is `Unknown` (`unknown_alias:p`). Every row asserts the status and the
//! complete set of diagnostic keys.
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

/// `return *(T * const)p;` through a null untracked `p`: the dereference
/// makes `p` a base of the read node through every qualifier spelling, a
/// parenthesised cast and two pointer levels. At the base every row was
/// `Safe`.
#[test]
fn a_null_untracked_root_through_a_qualified_cast_is_a_base() {
    if !clang_available() {
        return;
    }
    for (label, expression) in [
        ("const pointer", "*(long * const)p"),
        ("volatile pointer", "*(long * volatile)p"),
        ("restrict pointer", "*(long * restrict)p"),
        ("const volatile pointer", "*(const long * const volatile)p"),
        ("nullability qualifier", "*(long * _Nonnull)p"),
        ("parenthesised cast", "*((long * const)p)"),
    ] {
        expect(
            label,
            &format!("long w(void) {{ const long *p = 0; return {expression}; }}"),
            "Unknown",
            &["unknown_alias:p"],
        );
    }
    expect(
        "two pointer levels",
        "long w(void) { const long * const *pp = 0; return **(const long * const *)pp; }",
        "Unknown",
        &["unknown_alias:pp"],
    );
}

/// The readings the card keeps: an unbound formal discharges the base
/// obligation through a qualified cast, a tracked root keeps its verdict, a
/// value cast of the root is no dereference, and a second dereference of a
/// may-bind root through a qualified group keeps the return refusal (rule
/// 74h's undecided `*` refuses it, and rule 74m's count would).
#[test]
fn the_base_readings_are_kept() {
    if !clang_available() {
        return;
    }
    expect(
        "unbound formal",
        "long w(const long *p) { return *(long * const)p; }",
        "Safe",
        &[],
    );
    expect(
        "tracked non-null root",
        "long w(void) { long *p = k_alloc(8); if (!p) return 0; return *(long * const)p; }",
        "Unsafe",
        &["leak"],
    );
    expect(
        "tracked released root",
        "long w(void) { long *p = k_alloc(8); if (!p) return 0; k_free(p); \
         return *(long * const)p; }",
        "Unsafe",
        &["UseAfterRelease", "unknown_alias:p"],
    );
    expect(
        "value cast",
        "long w(void) { const char *s = 0; return (long)(const char * const)s; }",
        "Safe",
        &[],
    );
    for expression in ["*(long * const)*q", "*(long *)*q"] {
        expect(
            "second dereference of a may-bind root",
            &format!("long w(void **q) {{ return {expression}; }}"),
            "Unknown",
            &["unsupported:unmodeled call, aggregate or indirect access in return expression"],
        );
    }
}
