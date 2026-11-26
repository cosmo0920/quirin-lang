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

//! Card **RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT** (gateway rule 74y):
//! family S of W45-RETURN-EXPRESSION-FAMILIES before a root that fills a
//! subscript's brackets, end to end through the real gateway.
//!
//! C11 6.5.2.1p2 makes `E1[E2]` `(*((E1)+(E2)))`, so `return 1[p];`
//! dereferences `p` as `return p[1];` does. At the base family S read a root
//! as a subscript base only as `p [`, so through a null `p` the flow check
//! never saw the access and the function was `Safe`, while `return p[1];` is
//! `Unknown` (`unknown_alias:p`). A may-bind `q` in `n[*q]` was admitted as
//! one dereference. Every row asserts the status and the complete set of
//! diagnostic keys.
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

/// A null untracked `p` inside a subscript's brackets under an integer
/// subscripted operand: `p` is a base of the read node, as in `p[1]`. At the
/// base every row was `Safe`.
#[test]
fn a_null_untracked_root_inside_a_subscript_is_a_base() {
    if !clang_available() {
        return;
    }
    for (label, pointer, expression) in [
        ("literal outside", "long", "1[p]"),
        ("formal scalar outside", "long", "k[p]"),
        ("cast inside", "char", "k[(const long *)p]"),
        ("parenthesised root", "long", "k[(p)]"),
        ("sum inside, literal outside", "long", "0[p + k]"),
        ("sum inside, scalar outside", "long", "k[p + 1]"),
        ("cast sum inside", "char", "1[(const long *)p + k]"),
        ("parenthesised sum inside", "long", "0[(p + 1)]"),
        ("group outside", "long", "(k + 1)[p]"),
        ("member of the element", "struct s", "0[p].n"),
        ("element in a sum", "long", "k[p] + 1"),
    ] {
        expect(
            label,
            &format!("long w(long k) {{ const {pointer} *p = 0; return {expression}; }}"),
            "Unknown",
            &["unknown_alias:p"],
        );
    }
    expect(
        "local scalar outside",
        "long w(void) { const long *p = 0; long k = 1; return k[p]; }",
        "Unknown",
        &["unknown_alias:p"],
    );
}

/// A sum in the brackets whose other operand is another pointer expression,
/// or an element read: family S refuses, and the return refusal stands. At
/// the base both were `Safe`.
#[test]
fn an_unclassified_sum_in_the_brackets_refuses() {
    if !clang_available() {
        return;
    }
    for (label, expression) in [
        ("difference of pointers beside the root", "0[p + (q - q)]"),
        ("element read beside the root", "0[1[p] + p]"),
    ] {
        expect(
            label,
            &format!(
                "long w(void) {{ const long *p = 0; const long *q = 0; return {expression}; }}"
            ),
            REFUSED.0,
            REFUSED.1,
        );
    }
}

/// A may-bind `q` inside brackets whose subscripted operand is not a declared
/// pointer: `n[*q]` is `*(n + *q)`, a second dereference, refused as
/// `return **q;` is. At the base the first two were `Safe` and the released
/// one `Unsafe` (the flow check accused the first dereference).
#[test]
fn a_may_bind_root_inside_undecided_brackets_refuses() {
    if !clang_available() {
        return;
    }
    for (label, body) in [
        ("starred root", "long w(long **q, long n) { return n[*q]; }"),
        (
            "starred root in a sum",
            "long w(long **q, long n) { return n[*q + 1]; }",
        ),
        (
            "released starred root",
            "long w(void) { long **q = k_alloc(8); if (!q) return 0; k_free(q); \
             return 0[*q]; }",
        ),
    ] {
        expect(label, body, REFUSED.0, REFUSED.1);
    }
}

/// The readings the card keeps: the direct subscript of the null root, a
/// declared array or pointer subscripted by an untracked index that is no
/// scalar, unbound formals, an address, a `sizeof`, a tracked root and a
/// may-bind root under a declared array.
#[test]
fn the_base_readings_are_kept() {
    if !clang_available() {
        return;
    }
    expect(
        "direct subscript",
        "long w(void) { const long *p = 0; return p[1]; }",
        "Unknown",
        &["unknown_alias:p"],
    );
    expect(
        "declared array subscripted by a volatile index",
        "long w(void) { const long a[2] = {0, 0}; volatile long i = 0; return a[i]; }",
        "Safe",
        &[],
    );
    expect(
        "declared pointer subscripted by a volatile index",
        "long w(void) { const long *p = 0; volatile long i = 0; return p[i]; }",
        "Unknown",
        &["unknown_alias:p"],
    );
    for expression in ["k[p]", "1[p]", "k[(const long *)p + 1]"] {
        expect(
            "unbound formal",
            &format!("long w(const long *p, long k) {{ return {expression}; }}"),
            "Safe",
            &[],
        );
    }
    expect(
        "address of a reversed subscript",
        "long w(long k) { const long *p = 0; return &k[p] != 0; }",
        REFUSED.0,
        REFUSED.1,
    );
    expect(
        "sizeof of a reversed subscript",
        "long w(long k) { const long *p = 0; return sizeof(k[p]); }",
        "Safe",
        &[],
    );
    expect(
        "tracked root",
        "long w(void) { struct s *p = k_alloc(2 * sizeof *p); if (!p) return 0; \
         return 0[p].n; }",
        REFUSED.0,
        REFUSED.1,
    );
    expect(
        "may-bind root under a declared array",
        "long w(long *q) { const long a[2] = {0, 0}; return a[*q]; }",
        "Safe",
        &[],
    );
}

/// Family S refuses, and so does every older route: the return refusal.
const REFUSED: (&str, &[&str]) = (
    "Unknown",
    &["unsupported:unmodeled call, aggregate or indirect access in return expression"],
);
