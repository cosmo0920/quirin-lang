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

//! Card **RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST** (gateway rule 74t):
//! family S of W45-RETURN-EXPRESSION-FAMILIES before an untracked root that
//! `->` or `[` reaches through cast groups and parentheses, end to end
//! through the real gateway.
//!
//! At the base an untracked root was a member or subscript base of the read
//! node only when `->` or `[` came right after the name, so
//! `return ((const struct s *)p)->n;` through a null `p` made no base, the
//! flow check never saw the access, and the function was `Safe`, while
//! `return p->n;` is `Unknown` (`unknown_alias:p`). Every row asserts the
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

/// `return ((T *)p)->n;` and `return ((T *)p)[0];` through a null untracked
/// `p`: the access makes `p` a base of the read node, as `p->n` and `p[0]`
/// do, through one cast group, nested groups and parentheses. At the base
/// every row was `Safe`.
#[test]
fn a_null_untracked_member_or_subscript_root_through_a_cast_is_a_base() {
    if !clang_available() {
        return;
    }
    for (label, pointer, expression) in [
        ("member through a cast", "void", "((const struct s *)p)->n"),
        ("subscript through a cast", "void", "((const long *)p)[0]"),
        (
            "member through a cast, parenthesised root",
            "void",
            "((const struct s *)(p))->n",
        ),
        (
            "member through nested casts",
            "void",
            "((struct s *)(char *)p)->n",
        ),
        (
            "member through doubled parentheses",
            "void",
            "(((struct s *)p))->n",
        ),
        ("member then array", "void", "((struct s *)p)->a[1]"),
        ("subscript in a sum", "void", "((const long *)p)[1] + 1"),
        (
            "member under a cast",
            "void",
            "(int) ((const struct s *)p)->n",
        ),
        ("parenthesised root, member", "struct s", "(p)->n"),
        ("doubly parenthesised root, member", "struct s", "((p))->n"),
        ("parenthesised root, subscript", "long", "(p)[0]"),
    ] {
        expect(
            label,
            &format!("long w(void) {{ const {pointer} *p = 0; return {expression}; }}"),
            "Unknown",
            &["unknown_alias:p"],
        );
    }
}

/// The readings the card keeps: the direct member and subscript of the null
/// root (bases already), an unbound formal, whose base obligation discharges
/// through a cast as without one, an address and a `sizeof`, which
/// dereference nothing, and a tracked root.
#[test]
fn the_base_readings_are_kept() {
    if !clang_available() {
        return;
    }
    // Family S's direct spellings (`return p->n;` alone is the strict
    // `Returned` rule's, not family S's: registered follow-up).
    for (label, expression) in [
        ("direct member", "p->n + 1"),
        ("direct member then array", "p->a[0] + 1"),
    ] {
        expect(
            label,
            &format!("long w(void) {{ const struct s *p = 0; return {expression}; }}"),
            "Unknown",
            &["unknown_alias:p"],
        );
    }
    for expression in [
        "((const struct s *)p)->n",
        "((const long *)p)[0]",
        "((const struct s *)(p))->n",
    ] {
        expect(
            "unbound formal",
            &format!("long w(const void *p) {{ return {expression}; }}"),
            "Safe",
            &[],
        );
    }
    // No dereference: an address (family S refuses `&`) and a `sizeof`.
    expect(
        "address of a member through a cast",
        "long w(void) { const void *p = 0; return &((const struct s *)p)->n != 0; }",
        ADDRESS.0,
        ADDRESS.1,
    );
    expect(
        "sizeof of a member through a cast",
        "long w(void) { const void *p = 0; return sizeof(((const struct s *)p)->n); }",
        SIZEOF.0,
        SIZEOF.1,
    );
    // A tracked `p` is a may-bind root, which this card leaves as it was:
    // the typed return route decides a member through a cast as without one,
    // and an element through a cast keeps the return refusal.
    const REFUSED: (&str, &[&str]) = ADDRESS;
    for (expression, live, released) in [
        ("p->n", LEAK, AFTER_RELEASE),
        ("((struct s *)p)->n", LEAK, AFTER_RELEASE),
        ("(p)->n", LEAK, AFTER_RELEASE),
        ("((struct s *)p)->n + 1", LEAK, AFTER_RELEASE),
        ("(p)->n + 1", LEAK, AFTER_RELEASE),
        ("((long *)p)[0]", REFUSED, REFUSED),
        ("((long *)p)[0] + 1", REFUSED, REFUSED),
    ] {
        expect(
            "tracked non-null root",
            &format!(
                "long w(void) {{ struct s *p = k_alloc(sizeof *p); if (!p) return 0; \
                 return {expression}; }}"
            ),
            live.0,
            live.1,
        );
        expect(
            "tracked released root",
            &format!(
                "long w(void) {{ struct s *p = k_alloc(sizeof *p); if (!p) return 0; k_free(p); \
                 return {expression}; }}"
            ),
            released.0,
            released.1,
        );
    }
}

const LEAK: (&str, &[&str]) = ("Unsafe", &["leak"]);
const AFTER_RELEASE: (&str, &[&str]) = ("Unsafe", &["UseAfterRelease", "unknown_alias:p"]);

/// Family S refuses `&`, and so does every older route: the return refusal.
const ADDRESS: (&str, &[&str]) = (
    "Unknown",
    &["unsupported:unmodeled call, aggregate or indirect access in return expression"],
);
/// A `sizeof` operand is not evaluated.
const SIZEOF: (&str, &[&str]) = ("Safe", &[]);
