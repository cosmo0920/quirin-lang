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

//! Card **RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS** (gateway rule 74x):
//! family S of W45-RETURN-EXPRESSION-FAMILIES before a parenthesised
//! conditional that `->`, `[` or a unary `*` accesses, end to end through
//! the real gateway.
//!
//! At the base no test read a root that is an arm of the accessed
//! conditional, so `return (0 ? p : q)->n;` through null `p` and `q` made no
//! base, the flow check never saw the access, and the function was `Safe`,
//! while `return k ? p->n : q->n;` is `Unknown` (`unknown_alias:p`,
//! `unknown_alias:q`). Every row asserts the status and the complete set of
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

const BOTH: &[&str] = &["unknown_alias:p", "unknown_alias:q"];

/// A parenthesised conditional over null untracked `p` and `q`, then `->`,
/// `[` or `*`: both arms are bases of the read node under every condition
/// the reader does not decide. Rule 74z prunes under a decided one
/// (`return_family_conditional_constant_pruning.rs`). At the base every row
/// was `Safe`.
#[test]
fn both_null_arms_of_an_accessed_conditional_are_bases() {
    if !clang_available() {
        return;
    }
    for (label, pointer, expression, keys) in [
        (
            "member, undecided zero",
            "struct s",
            "(k - k ? p : q)->n",
            BOTH,
        ),
        (
            "member, undecided sizeof",
            "struct s",
            "(sizeof(int) - 3 ? p : q)->n",
            BOTH,
        ),
        ("member", "struct s", "(k ? p : q)->n + 1", BOTH),
        ("element", "long", "(k ? p : q)[0]", BOTH),
        ("dereference", "long", "*(k ? p : q)", BOTH),
        (
            "nested",
            "struct s",
            "(k ? (k > 1 ? p : r) : q)->n",
            &["unknown_alias:p", "unknown_alias:q", "unknown_alias:r"],
        ),
        (
            "nested, unparenthesised",
            "struct s",
            "(k ? p : k > 1 ? q : r)->n",
            &["unknown_alias:p", "unknown_alias:q", "unknown_alias:r"],
        ),
        (
            "cast around",
            "void",
            "((const struct s *)(k ? p : q))->n",
            BOTH,
        ),
        (
            "dereference of a cast",
            "void",
            "*(const long *)(k ? p : q)",
            BOTH,
        ),
        ("sum in an arm", "struct s", "(k ? p + 1 : q)->n", BOTH),
        ("sum around", "struct s", "((k ? p : q) + 1)->n", BOTH),
        (
            "null-constant arm",
            "struct s",
            "(k ? p : 0)->n",
            &["unknown_alias:p"],
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
    expect(
        "cast in an arm",
        "long w(long k) { const void *p = 0; const struct s *q = 0; \
         return (k ? (const struct s *)p : q)->n; }",
        "Unknown",
        BOTH,
    );
    // A loaded arm's root is the old tests' (here an unbound formal): the
    // other arm is still a base.
    expect(
        "member chain arm, formal root",
        "long w(long k, const struct s *a) { const struct s *q = 0; \
         return (k ? a->next : q)->n; }",
        "Unknown",
        &["unknown_alias:q"],
    );
    // The condition is a read: `p` is no base, the accessed arms are.
    expect(
        "root in the condition",
        "long w(void) { const struct s *p = 0; const struct s *q = 0; \
         const struct s *r = 0; return (p ? q : r)->n; }",
        "Unknown",
        &["unknown_alias:q", "unknown_alias:r"],
    );
}

/// An arm family S cannot classify refuses the expression, which then falls
/// to the later routes: rule 10z lowers the string arm by branching, and its
/// `p[0]` edge reads `p` (at the base `Safe`).
#[test]
fn an_unclassifiable_arm_refuses() {
    if !clang_available() {
        return;
    }
    expect(
        "string arm",
        "long w(long k) { const char *p = 0; return (k ? p : \"ab\")[0]; }",
        "Unknown",
        &["unknown_alias:p"],
    );
}

/// The readings the card keeps: arms already accessed, a comparison, unbound
/// formals, a root only in the condition, an address, a `sizeof`, a tracked
/// root and the call and comma refusals.
#[test]
fn the_base_readings_are_kept() {
    if !clang_available() {
        return;
    }
    let two = "const struct s *p = 0; const struct s *q = 0;";
    for (label, expression) in [
        ("arms already accessed", "k ? p->n : q->n"),
        ("arms already accessed, parenthesised", "(k ? p->n : q->n)"),
    ] {
        expect(
            label,
            &format!("long w(long k) {{ {two} return {expression}; }}"),
            "Unknown",
            BOTH,
        );
    }
    expect(
        "comparison",
        &format!("long w(long k) {{ {two} return (k ? p : q) != 0; }}"),
        "Safe",
        &[],
    );
    expect(
        "sizeof",
        &format!("long w(long k) {{ {two} return sizeof((k ? p : q)->n); }}"),
        "Safe",
        &[],
    );
    for expression in ["(k ? p : q)->n", "(0 ? p : q)[0].n"] {
        expect(
            "unbound formals",
            &format!(
                "long w(long k, const struct s *p, const struct s *q) {{ return {expression}; }}"
            ),
            "Safe",
            &[],
        );
    }
    expect(
        "root only in the condition, formal arms",
        "long w(const struct s *q, const struct s *r) { const struct s *p = 0; \
         return (p ? q : r)->n; }",
        "Safe",
        &[],
    );
    for (label, body) in [
        (
            "address",
            format!("long w(long k) {{ {two} return &(k ? p : q)->n != 0; }}"),
        ),
        (
            "tracked root",
            "long w(long k) { struct s *p = k_alloc(sizeof *p); if (!p) return 0; \
             return (k ? p : p)->n; }"
                .to_owned(),
        ),
        (
            "comma arm",
            format!("long w(long k) {{ {two} return (k ? (k, p) : q)->n; }}"),
        ),
        (
            "call arm",
            format!("long w(long k) {{ {two} return (k ? (const struct s *)k_alloc(8) : q)->n; }}"),
        ),
    ] {
        expect(label, &body, REFUSED.0, REFUSED.1);
    }
}

/// Family S refuses, and so does every older route: the return refusal.
const REFUSED: (&str, &[&str]) = (
    "Unknown",
    &["unsupported:unmodeled call, aggregate or indirect access in return expression"],
);
