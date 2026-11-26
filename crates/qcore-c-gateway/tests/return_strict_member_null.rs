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

//! Card **RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT** (gateway rule 74v): card
//! B01's opaque member read in `return` position, asked before family S,
//! keeps family S's untracked-root base, end to end through the real gateway.
//!
//! At the base `return p->n;` through a null untracked `p` lowered to an
//! `OpaqueValue` into the return place with no obligation for the root `->`
//! dereferences, so the function was `Safe`, while `return p->n + 1;`
//! (family S) and `long x = p->n; return x;` are `Unknown`
//! (`unknown_alias:p`). Every row asserts the status and the complete set of
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
struct t { long b; }; \
struct s { long n; long a[2]; struct s *next; struct t inner; struct s *q; };";

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
            eprintln!("skipping Clang-dependent strict-member test: {reason}");
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

/// Every row of one test is checked before it fails, so a failure lists each
/// row that differs (`label`, body, measured, expected).
struct Rows(Vec<String>);

impl Rows {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn expect(&mut self, label: &str, body: &str, status: &str, vector: &[&str]) {
        let (measured, keys) = outcome(body);
        let keys = keys.iter().map(String::as_str).collect::<Vec<_>>();
        if (measured, keys.clone()) != (status, vector.to_vec()) {
            self.0.push(format!(
                "{label}: {body}\n  measured {measured} {keys:?}, expected {status} {vector:?}"
            ));
        }
    }

    fn check(self) {
        assert!(
            self.0.is_empty(),
            "{} rows differ:\n{}",
            self.0.len(),
            self.0.join("\n")
        );
    }
}

/// The bare member read through a null untracked root: B01's spellings, a
/// scalar and a pointer result, with and without the cast `cast_return`
/// strips first. At the base every row was `Safe`.
#[test]
fn a_bare_member_read_through_a_null_untracked_root_is_refused() {
    if !clang_available() {
        return;
    }
    let mut rows = Rows::new();
    for (label, body) in [
        (
            "scalar member, const root",
            "long w(void) { const struct s *p = 0; return p->n; }",
        ),
        (
            "scalar member, mutable root",
            "long w(void) { struct s *p = 0; return p->n; }",
        ),
        (
            "scalar member, null spelled as a cast",
            "long w(void) { const struct s *p = (void *)0; return p->n; }",
        ),
        (
            "pointer member",
            "struct s *w(void) { const struct s *p = 0; return p->next; }",
        ),
        (
            "pointer member under a pointer cast",
            "void *w(void) { const struct s *p = 0; return (void *)p->next; }",
        ),
    ] {
        rows.expect(label, body, "Unknown", &["unknown_alias:p"]);
    }
    rows.check();
}

/// For every root binding, the bare read now gets the verdict family S gives
/// `p->n + 1`: refused for a null root, admitted for an unbound formal, an
/// uninitialised local, a formal's copy, a member read and a root a null
/// test guards.
#[test]
fn the_bare_read_gets_family_s_verdict() {
    if !clang_available() {
        return;
    }
    let mut rows = Rows::new();
    for (label, signature, setup, verdict) in [
        ("null local", "void", "const struct s *p = 0;", REFUSED_NULL),
        ("unbound formal", "const struct s *p", "", SAFE),
        ("uninitialised local", "void", "const struct s *p;", SAFE),
        (
            "copy of a formal",
            "const struct s *f",
            "const struct s *p = f;",
            SAFE,
        ),
        (
            "member of a formal",
            "const struct s *f",
            "const struct s *p = f->next;",
            SAFE,
        ),
        (
            "null test guards the read",
            "void",
            "const struct s *p = 0; if (!p) return 0;",
            SAFE,
        ),
    ] {
        for expression in ["p->n", "p->n + 1"] {
            rows.expect(
                label,
                &format!("long w({signature}) {{ {setup} return {expression}; }}"),
                verdict.0,
                verdict.1,
            );
        }
    }
    rows.expect(
        "pointer member of an unbound formal",
        "struct s *w(struct s *p) { return p->next; }",
        "Safe",
        &[],
    );
    rows.check();
}

/// The readings the card keeps: the spellings older cards already route to
/// family S or the declaration route, the address and the record value,
/// which dereference nothing, and a tracked root, which B01 never admits.
#[test]
fn the_base_readings_are_kept() {
    if !clang_available() {
        return;
    }
    let mut rows = Rows::new();
    for (label, pointer, expression) in [
        ("subscript", "long", "p[0]"),
        ("star", "long", "*p"),
        ("member through a cast", "void", "((const struct s *)p)->n"),
        ("parenthesised root", "struct s", "(p)->n"),
        ("parenthesised member", "struct s", "(p->n)"),
        ("member of a member", "struct s", "p->inner.b"),
        ("member of a link", "struct s", "p->q->n"),
        ("member then array", "struct s", "p->a[0]"),
        ("member under an integer cast", "struct s", "(long)p->n"),
    ] {
        rows.expect(
            label,
            &format!("long w(void) {{ const {pointer} *p = 0; return {expression}; }}"),
            REFUSED_NULL.0,
            REFUSED_NULL.1,
        );
    }
    rows.expect(
        "declaration route",
        "long w(void) { const struct s *p = 0; long x = p->n; return x; }",
        REFUSED_NULL.0,
        REFUSED_NULL.1,
    );
    rows.expect(
        "address of a member",
        "const long *w(void) { const struct s *p = 0; return &p->n; }",
        "Unknown",
        &["unsupported:unmodeled call, aggregate or indirect access in return expression"],
    );
    rows.expect(
        "member of a record value",
        "long w(void) { struct s v = {0}; return v.n; }",
        "Safe",
        &[],
    );
    rows.expect(
        "tracked non-null root",
        "long w(void) { struct s *p = k_alloc(sizeof *p); if (!p) return 0; return p->n; }",
        "Unsafe",
        &["leak"],
    );
    rows.expect(
        "tracked released root",
        "long w(void) { struct s *p = k_alloc(sizeof *p); if (!p) return 0; k_free(p); \
         return p->n; }",
        "Unsafe",
        &["UseAfterRelease", "unknown_alias:p"],
    );
    rows.expect(
        "tracked root read, released, value returned",
        "long w(void) { struct s *p = k_alloc(sizeof *p); long n = 0; if (!p) return 0; \
         n = p->n; k_free(p); return n; }",
        "Safe",
        &[],
    );
    rows.check();
}

const SAFE: (&str, &[&str]) = ("Safe", &[]);
const REFUSED_NULL: (&str, &[&str]) = ("Unknown", &["unknown_alias:p"]);
