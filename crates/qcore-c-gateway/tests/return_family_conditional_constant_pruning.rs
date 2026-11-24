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

//! Card **RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING** (gateway rule 74z):
//! family S of W45-RETURN-EXPRESSION-FAMILIES before an accessed
//! conditional whose condition is an integer constant expression the reader
//! evaluates exactly, end to end through the real gateway.
//!
//! At the base rule 74x made both arms bases whatever the condition, so
//! `long w(const struct s *q) { const struct s *p = 0; return (0 ? p : q)->n; }`
//! was `Unknown` (`unknown_alias:p`), though C never evaluates `p` (C11
//! 6.5.15p4). Every row asserts the status and the complete set of
//! diagnostic keys.
//!
//! The fixtures use the generic `k_*` surface of
//! `fixtures/contracts/descriptor_generic.qcore`; `Safe` here is `Safe` over
//! those synthetic trusted test assumptions.

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

const P: &[&str] = &["unknown_alias:p"];

/// `p` a null local, `q` and `r` unbound formals.
fn null_p(expression: &str) -> String {
    format!(
        "long w(long k, const struct s *q, const struct s *r) {{ \
         const struct s *p = 0; return {expression}; }}"
    )
}

/// The admitted fixtures: a decided condition drops the null `p` of the arm
/// C never evaluates, and the evaluated arm is an unbound formal. At the
/// base every row was `Unknown` (`unknown_alias:p`).
#[test]
fn a_decided_condition_prunes_the_unevaluated_arm() {
    if !clang_available() {
        return;
    }
    for (label, expression) in [
        ("constant false", "(0 ? p : q)->n"),
        ("constant true", "(1 ? q : p)->n"),
        ("parenthesised difference", "((2 - 2) ? p : q)->n"),
        ("unsigned comparison", "(-1 < 0u ? p : q)->n"),
        ("member subscript", "(0 ? p : q)->a[0]"),
        ("cast around", "((const struct s *)(0 ? p : q))->n"),
        ("decided inner, undecided outer", "(k ? (0 ? p : r) : q)->n"),
        ("decided outer, nested else", "(0 ? p : 1 ? q : p)->n"),
    ] {
        expect(label, &null_p(expression), "Safe", &[]);
    }
    // A macro is expanded before the reader sees the tokens, so a macro
    // that expands to `0` is read as the literal `0` C compiles in this
    // translation unit (report §2: a user decision is requested).
    expect(
        "macro expanding to a literal",
        "\n#define N 0\nlong w(const struct s *q) { const struct s *p = 0; \
         return (N ? p : q)->n; }",
        "Safe",
        &[],
    );
}

/// The refused fixtures: every condition the reader does not decide keeps
/// rule 74x's reading, both arms, so the null `p` stays a base.
#[test]
fn an_undecided_condition_keeps_both_arms() {
    if !clang_available() {
        return;
    }
    for (label, expression) in [
        ("formal condition", "(k ? p : q)->n"),
        ("sizeof", "(sizeof(int) - 4 ? p : q)->n"),
        ("division by zero", "(1 / 0 ? p : q)->n"),
        ("cast", "((int)0 ? p : q)->n"),
        ("signed overflow", "(2147483647 + 1 ? p : q)->n"),
        ("shift out of range", "(1 << 32 ? p : q)->n"),
        ("nested, inner undecided", "(1 ? (k ? p : r) : q)->n"),
    ] {
        expect(label, &null_p(expression), "Unknown", P);
    }
    expect(
        "difference of a local",
        "long w(const struct s *q) { int x = 0; const struct s *p = 0; \
         return (x - x ? p : q)->n; }",
        "Unknown",
        P,
    );
    // The reader reads the preprocessed token stream, so a macro's name
    // never reaches it: a macro that expands to a name is a name.
    expect(
        "macro expanding to a name",
        "\n#define N k\nlong w(long k, const struct s *q) { const struct s *p = 0; \
         return (N ? p : q)->n; }",
        "Unknown",
        P,
    );
    expect(
        "enumeration constant",
        "enum { N = 0 }; long w(const struct s *q) { const struct s *p = 0; \
         return (N ? p : q)->n; }",
        "Unknown",
        P,
    );
}

/// What pruning keeps: the evaluated arm's base, an old test's base inside
/// the untaken arm, and the arms of a conditional nothing accesses.
#[test]
fn pruning_keeps_every_other_obligation() {
    if !clang_available() {
        return;
    }
    for (label, expression) in [
        ("evaluated arm null", "(1 ? p : q)->n"),
        ("old test in the untaken arm", "(0 ? p->next : q)->n"),
        ("unaccessed conditional, arms accessed", "0 ? p->n : q->n"),
    ] {
        expect(label, &null_p(expression), "Unknown", P);
    }
}
