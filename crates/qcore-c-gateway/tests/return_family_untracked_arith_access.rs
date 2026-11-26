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

//! Card **RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC** (gateway rule
//! 74w): family S of W45-RETURN-EXPRESSION-FAMILIES before an untracked root
//! that `->`, `[` or a unary `*` reaches through pointer arithmetic, end to
//! end through the real gateway.
//!
//! At the base the direct tests and rules 74r's and 74t's walks stopped at
//! the `+` or `-`, so `return (p + 1)->n;` through a null `p` made no base,
//! the flow check never saw the access, and the function was `Safe`, while
//! `return p->n + 1;` is `Unknown` (`unknown_alias:p`). Every row asserts the
//! status and the complete set of diagnostic keys.
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

/// Pointer arithmetic on a null untracked `p`, then `->`, `[` or `*`: the
/// access makes `p` a base of the read node, as `p->n`, `p[0]` and `*p` do.
/// At the base every row was `Safe`.
#[test]
fn a_null_untracked_root_accessed_through_pointer_arithmetic_is_a_base() {
    if !clang_available() {
        return;
    }
    for (label, pointer, expression) in [
        ("member of a sum", "struct s", "(p + 1)->n"),
        ("member of a sum, index first", "struct s", "(k + p)->n"),
        ("member of a difference", "struct s", "(p - k)->n"),
        (
            "member of a cast sum",
            "void",
            "((const struct s *)p + 1)->n",
        ),
        (
            "member of a cast of a sum",
            "char",
            "((const struct s *)(p + k))->n",
        ),
        ("element of a sum", "long", "(p + 1)[0]"),
        ("element of a sum, index first", "long", "(1 + p)[k]"),
        ("nested sums", "long", "((p + 1) + k)[0]"),
        ("sum in a sum", "long", "(p + (k * 2))[0] + 1"),
        (
            "sum with a sizeof",
            "char",
            "((const struct s *)(p + sizeof(long)))->n",
        ),
        ("dereference of a sum, index first", "long", "*(k + p)"),
        (
            "dereference of a cast sum",
            "char",
            "*(const long *)(k + p)",
        ),
    ] {
        expect(
            label,
            &format!("long w(long k) {{ const {pointer} *p = 0; return {expression}; }}"),
            "Unknown",
            &["unknown_alias:p"],
        );
    }
}

/// An access through a sum whose other operand is another pointer: family S
/// refuses, and the return refusal stands. `(p - q)[a]` subscripts the array
/// `a` by a difference; the reader cannot tell which name is the pointer, so
/// it refuses rather than guess (at the base these were `Safe`, which is
/// right for a real array `a`: a precision loss the rule accepts).
#[test]
fn an_access_through_an_unclassified_sum_refuses() {
    if !clang_available() {
        return;
    }
    for expression in ["(p - q)[a]", "(a + (p - q))[0]"] {
        expect(
            "difference of two pointers",
            &format!(
                "long w(void) {{ const long *p = 0; const long *q = 0; \
                 const long a[2] = {{0, 0}}; return {expression}; }}"
            ),
            REFUSED.0,
            REFUSED.1,
        );
    }
}

/// The readings the card keeps: the direct and starred spellings of the null
/// root (bases already), an unbound formal, a value and a comparison (no
/// access), an address and a `sizeof`, and a tracked root.
#[test]
fn the_base_readings_are_kept() {
    if !clang_available() {
        return;
    }
    for (label, pointer, expression) in [
        ("direct member", "struct s", "p->n + 1"),
        ("dereference of a sum, root first", "long", "*(p + 1)"),
        ("member through a cast", "void", "((const struct s *)p)->n"),
    ] {
        expect(
            label,
            &format!("long w(void) {{ const {pointer} *p = 0; return {expression}; }}"),
            "Unknown",
            &["unknown_alias:p"],
        );
    }
    for expression in ["(p + 1)->n", "(k + p)[0].n", "((const struct s *)p + k)->n"] {
        expect(
            "unbound formal",
            &format!("long w(const struct s *p, long k) {{ return {expression}; }}"),
            "Safe",
            &[],
        );
    }
    // No access: a comparison of the sum, and the sum as a value (not family
    // S's: a pointer-valued result keeps its route).
    expect(
        "comparison of a sum",
        "long w(void) { const struct s *p = 0; return (p + 1) != 0; }",
        "Safe",
        &[],
    );
    expect(
        "sum as a value",
        "const struct s *w(void) { const struct s *p = 0; return p + 1; }",
        VALUE.0,
        VALUE.1,
    );
    // No dereference: an address (family S refuses `&`) and a `sizeof`.
    expect(
        "address of a member of a sum",
        "long w(void) { const struct s *p = 0; return &(p + 1)->n != 0; }",
        REFUSED.0,
        REFUSED.1,
    );
    expect(
        "sizeof of a member of a sum",
        "long w(void) { const struct s *p = 0; return sizeof((p + 1)->n); }",
        "Safe",
        &[],
    );
    // A tracked `p` is a may-bind root, which this card leaves as it was.
    for expression in ["(p + 1)->n", "(p + 1)[0].n", "(1 + p)->n + 1"] {
        expect(
            "tracked non-null root",
            &format!(
                "long w(void) {{ struct s *p = k_alloc(2 * sizeof *p); if (!p) return 0; \
                 return {expression}; }}"
            ),
            TRACKED.0,
            TRACKED.1,
        );
    }
}

/// Family S refuses, and so does every older route: the return refusal.
const REFUSED: (&str, &[&str]) = (
    "Unknown",
    &["unsupported:unmodeled call, aggregate or indirect access in return expression"],
);
/// `return p + 1;` in a pointer-result function is not family S's; its own
/// route's reading, the same at the base.
const VALUE: (&str, &[&str]) = ("Safe", &[]);
/// A tracked root through arithmetic is a may-bind root that family S
/// refuses, as at the base, and the return refusal stands.
const TRACKED: (&str, &[&str]) = REFUSED;
