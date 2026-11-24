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

//! Card **RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST** (gateway rule 74h):
//! family S of W45-RETURN-EXPRESSION-FAMILIES after a `)` its declaration
//! reader does not resolve, end to end through the real gateway.
//!
//! `(volatile long)` (a rejecting qualifier) and `(k_lp)` (a pointer
//! typedef) are type names the reader leaves unresolved. At the base family S
//! read the `*` after them as a multiplication, so an untracked `p` was no
//! base and `return (volatile long) *p;` through a null `p` was `Safe`. Every
//! row asserts the status and the complete set of diagnostic keys.
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
int z_unknown(int n); \
struct k_box { void *item; long n; }; \
typedef enum { K_READ, K_WRITE } k_kind_t; \
typedef long *k_lp;";

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

/// `return (T) *p;` with `T` unresolved and `p` untracked: `p` is a base of
/// the read node, so the flow check refuses the null entry binding exactly
/// as it does for the resolved cast and the bare dereference. At the base
/// both unresolved rows were `Safe`.
#[test]
fn an_unresolved_cast_before_a_dereference_keeps_the_base() {
    if !clang_available() {
        return;
    }
    for (label, body) in [
        (
            "rejecting qualifier",
            "long w(void) { const long *p = 0; return (volatile long) *p; }",
        ),
        (
            "pointer typedef",
            "long w(void) { const long *p = 0; return (k_lp) *p != 0; }",
        ),
        (
            "resolved cast (control)",
            "long w(void) { const long *p = 0; return (long) *p; }",
        ),
        (
            "bare dereference (control)",
            "long w(void) { const long *p = 0; return *p; }",
        ),
    ] {
        expect(label, body, "Unknown", &["unknown_alias:p"]);
    }
    // An untracked base whose binding the flow check accepts stays decided.
    expect(
        "opaque field read",
        "long w(const struct k_box *b) { const long *p = b->item; \
         return (volatile long) *p; }",
        "Safe",
        &[],
    );
}

/// `return (T) *q;` with `T` unresolved and `q` may-bind: family S keeps the
/// operand reading's refusal (it never admits `q` as a dereference base
/// through an undecided group), and the typed return route decides the
/// dereference from Clang's AST, as at the base.
#[test]
fn a_may_bind_root_after_an_unresolved_cast_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    for body in [
        "long w(void) { long *q = k_alloc(8); if (!q) return 0; k_free(q); \
         return (volatile long) *q; }",
        "long w(void) { long *q = k_alloc(8); if (!q) return 0; k_free(q); \
         return (long) *q; }",
        // One dereference as an operand, two as a cast: refused by family S.
        "long w(void) { long *q = k_alloc(8); if (!q) return 0; k_free(q); \
         return (K_READ) * *q; }",
    ] {
        expect(
            "released may-bind root",
            body,
            "Unsafe",
            &["UseAfterRelease", "unknown_alias:q"],
        );
    }
    expect(
        "released formal",
        "long w(long *q) { k_free(q); return (volatile long) *q; }",
        "Unknown",
        &["unknown_alias:q"],
    );
}

/// Binary controls: a call result and a declared object before `*` or `&`.
#[test]
fn an_operand_group_before_a_star_stays_binary() {
    if !clang_available() {
        return;
    }
    expect(
        "declared object times",
        "long w(long a, long b) { return (a) * b; }",
        "Safe",
        &[],
    );
    expect(
        "declared object and",
        "long w(long a, long b) { return (a) & b; }",
        "Safe",
        &[],
    );
    expect(
        "call result times",
        "long w(long n) { return z_unknown(1) * n; }",
        "Unknown",
        &[RETURN],
    );
}
