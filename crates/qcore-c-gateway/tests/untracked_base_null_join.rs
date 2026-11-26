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

//! Card **UNTRACKED-BASE-NULL-JOIN-DISCHARGE** (gateway rule 83): a binding
//! that is the null constant on some reaching path is never rule 14n's
//! premised alias, end to end through the real gateway.
//!
//! At the base, `const struct s *p = 0; if (f) p = f; return p->n + 1;` was
//! `Safe` under the premise `null-deref:w:LINE:p`: the pointer join made the
//! null constant joined with an unbound copy `unknown` and possibly null,
//! rule 14n's premised shape. `const struct s *p = 0; return p->n + 1;` is
//! `Unknown` (`unknown_alias:p`). Every row asserts the status, the complete
//! set of diagnostic keys and the null premises the row rests on.
//!
//! The fixtures use the generic `k_*` surface of
//! `fixtures/contracts/descriptor_generic.qcore` plus one declared non-null
//! owned result (`k_make`); `Safe` here is `Safe` over those synthetic
//! trusted test assumptions.

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
void *k_make(void); \
struct s { long n; long a[2]; struct s *next; };";

const MAKE: &str = "module contract;
extern fn k_make() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nonnull);
};
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "descriptor_generic.qcore".to_owned(),
            include_str!("fixtures/contracts/descriptor_generic.qv").to_owned(),
        ),
        ("make.qcore".to_owned(), MAKE.to_owned()),
    ]))
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
            eprintln!("skipping Clang-dependent null-join test: {reason}");
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

/// The status of `w`, the sorted, de-duplicated keys of its diagnostics and
/// the sorted, de-duplicated variables of its null premises.
fn outcome(body: &str) -> (&'static str, Vec<String>, Vec<String>) {
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
    let mut premises = function
        .null_premises
        .iter()
        .map(|premise| premise.variable.clone())
        .collect::<Vec<_>>();
    premises.sort();
    premises.dedup();
    (status, keys, premises)
}

/// Every row of one test is checked before it fails, so a failure lists each
/// row that differs (`label`, body, measured, expected).
struct Rows(Vec<String>);

impl Rows {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn expect(&mut self, label: &str, body: &str, status: &str, keys: &[&str], premises: &[&str]) {
        let (measured, got, held) = outcome(body);
        let got = got.iter().map(String::as_str).collect::<Vec<_>>();
        let held = held.iter().map(String::as_str).collect::<Vec<_>>();
        if (measured, got.clone(), held.clone()) != (status, keys.to_vec(), premises.to_vec()) {
            self.0.push(format!(
                "{label}: {body}\n  measured {measured} {got:?} premises {held:?}, \
                 expected {status} {keys:?} premises {premises:?}"
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

/// The registered follow-up's three spellings: family S's `p->n + 1`, the
/// bare `p->n` (rule 74v routes it to family S's node) and the declaration
/// route. At the base each was `Safe` under the premise on `p`.
#[test]
fn the_registered_spellings_are_refused() {
    if !clang_available() {
        return;
    }
    let mut rows = Rows::new();
    let head = "long w(const struct s *f) { const struct s *p = 0; if (f) p = f;";
    for (label, tail) in [
        ("family S", "return p->n + 1; }"),
        ("bare member read", "return p->n; }"),
        ("declaration", "long x = p->n; return x; }"),
    ] {
        rows.expect(
            label,
            &format!("{head} {tail}"),
            "Unknown",
            &["unknown_alias:p"],
            &[],
        );
    }
    rows.check();
}

/// The same mechanism under other joins and other access shapes. At the
/// base every row but the last was `Safe` under a premise on the accessed
/// name; the ternary was already `Unknown` (on `f`) and now also names `p`.
#[test]
fn every_census_shape_is_refused() {
    if !clang_available() {
        return;
    }
    let mut rows = Rows::new();
    for (label, body) in [
        (
            "join with no guard",
            "long w(const struct s *f, int c) { const struct s *p = 0; if (c) p = f; return p->n + 1; }",
        ),
        (
            "inverse guard",
            "long w(const struct s *f) { const struct s *p; if (!f) p = 0; else p = f; return p->n + 1; }",
        ),
        (
            "inverse guard over a member",
            "long w(const struct s *f) { const struct s *p; if (!f) p = 0; else p = f->next; return p->n + 1; }",
        ),
        (
            "joined with a member of a formal",
            "long w(const struct s *f, int c) { const struct s *p = 0; if (c) p = f->next; return p->n + 1; }",
        ),
        (
            "joined with a guarded member",
            "long w(const struct s *f) { const struct s *p = 0; if (f->next) p = f->next; return p->n + 1; }",
        ),
        (
            "statement read",
            "long w(const struct s *f) { const struct s *p = 0; long x; if (f) p = f; x = p->n; return x; }",
        ),
        (
            "condition",
            "long w(const struct s *f) { const struct s *p = 0; if (f) p = f; if (p->n) return 1; return 0; }",
        ),
        (
            "store through the base",
            "void w(struct s *f) { struct s *p = 0; if (f) p = f; p->n = 1; }",
        ),
        (
            "subscript",
            "long w(const long *f) { const long *p = 0; if (f) p = f; return p[0] + 1; }",
        ),
        (
            "star",
            "long w(const long *f) { const long *p = 0; if (f) p = f; return *p + 1; }",
        ),
        (
            "member chain",
            "long w(const struct s *f) { const struct s *p = 0; if (f) p = f; return p->next->n + 1; }",
        ),
    ] {
        rows.expect(label, body, "Unknown", &["unknown_alias:p"], &[]);
    }
    rows.expect(
        "a copy carries the null",
        "long w(const struct s *f) { const struct s *p = 0; if (f) p = f; const struct s *q = p; return q->n + 1; }",
        "Unknown",
        &["unknown_alias:q"],
        &[],
    );
    rows.expect(
        "ternary join (already refused on f)",
        "long w(const struct s *f, int c) { const struct s *p = c ? 0 : f; return p->n + 1; }",
        "Unknown",
        &["unknown_alias:f", "unknown_alias:p"],
        &[],
    );
    rows.check();
}

/// Bindings with no null constant on any reaching path, and the guards that
/// already decide, keep their base verdicts (and their premises).
#[test]
fn guards_and_null_free_bindings_keep_their_verdicts() {
    if !clang_available() {
        return;
    }
    let mut rows = Rows::new();
    for (label, body) in [
        (
            "a copy of an unbound formal",
            "long w(const struct s *f) { const struct s *p = f; return p->n + 1; }",
        ),
        (
            "an early null test of the copy, family S",
            "long w(const struct s *f) { const struct s *p = f; if (!p) return 0; return p->n + 1; }",
        ),
        (
            "an early null test of the copy, bare read",
            "long w(const struct s *f) { const struct s *p = f; if (!p) return 0; return p->n; }",
        ),
        (
            "a dominating test of the copy",
            "long w(const struct s *f) { const struct s *p = f; if (p) return p->n + 1; return 0; }",
        ),
        (
            "the read inside the assigning arm",
            "long w(const struct s *f) { const struct s *p = 0; if (f) { p = f; return p->n + 1; } return 0; }",
        ),
        (
            "every arm overwrites the null",
            "long w(const struct s *f, int c) { const struct s *p = 0; if (c) p = f; else p = f; return p->n + 1; }",
        ),
        (
            "a declared non-null result on every path",
            "long w(void) { struct s *p = 0; p = k_make(); p->n = 1; long r = p->n; k_free(p); return r; }",
        ),
        (
            "an allocation tested before use",
            "long w(void) { struct s *p = 0; p = k_alloc(sizeof *p); if (!p) return 0; p->n = 1; long r = p->n; k_free(p); return r; }",
        ),
    ] {
        rows.expect(label, body, "Safe", &[], &[]);
    }
    for (label, body) in [
        (
            "a member read of a formal",
            "long w(const struct s *f) { const struct s *p = f->next; return p->n + 1; }",
        ),
        (
            "the null overwritten by a member read",
            "long w(const struct s *f) { const struct s *p = 0; p = f->next; return p->n + 1; }",
        ),
        (
            "both arms overwrite with a member read",
            "long w(const struct s *f, int c) { const struct s *p = 0; if (c) p = f->next; else p = f; return p->n + 1; }",
        ),
    ] {
        rows.expect(label, body, "Safe", &[], &["p"]);
    }
    for (label, body) in [
        (
            "the null constant alone",
            "long w(void) { const struct s *p = 0; return p->n + 1; }",
        ),
        (
            "a dominating test of the joined binding",
            "long w(const struct s *f) { const struct s *p = 0; if (f) p = f; if (p) return p->n + 1; return 0; }",
        ),
        (
            "a dominating test of the joined binding, bare read",
            "long w(const struct s *f) { const struct s *p = 0; if (f) p = f; if (p) return p->n; return 0; }",
        ),
        (
            "an early test of the joined binding",
            "long w(const struct s *f) { const struct s *p = 0; if (f) p = f; if (!p) return 0; return p->n + 1; }",
        ),
    ] {
        rows.expect(label, body, "Unknown", &["unknown_alias:p"], &[]);
    }
    rows.check();
}

/// Near misses: a test of another name, a test before a later null, a test
/// inside a loop, and a test of the formal the binding is correlated with,
/// which this reader does not relate to `p` (a precision loss, rule 83's
/// follow-up). The first and the last were `Safe` at the base.
#[test]
fn near_misses_are_refused() {
    if !clang_available() {
        return;
    }
    let mut rows = Rows::new();
    for (label, body) in [
        (
            "a test of a different name",
            "long w(const struct s *f, const struct s *g) { const struct s *p = 0; if (f) p = f; if (!g) return 0; return p->n + 1; }",
        ),
        (
            "a test before a later null",
            "long w(const struct s *f) { const struct s *p = f; if (!p) return 0; p = 0; return p->n + 1; }",
        ),
        (
            "a test inside a loop",
            "long w(const struct s *f, int c) { const struct s *p = 0; while (c--) { if (f) p = f; } return p->n + 1; }",
        ),
        (
            "a test of the correlated formal",
            "long w(const struct s *f) { const struct s *p = 0; if (f) p = f; if (!f) return 0; return p->n + 1; }",
        ),
    ] {
        rows.expect(label, body, "Unknown", &["unknown_alias:p"], &[]);
    }
    rows.check();
}
