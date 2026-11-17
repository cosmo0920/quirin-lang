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

//! Card **CG25-VOID-CAST-GUARD-ESCAPE** (gateway rule 11v (6)): every mention
//! of a tracked in-place object, whichever route lowers it.
//!
//! Each shape runs over a tracked `regex_t re`, compiled with its status
//! tested, once before `k_regfree(&re)` (**pre**) and once after it
//! (**post**). The escapes of the census
//! (`docs/reports/cg25-void-cast-guard-escape/README.md` §1) were certified
//! `Safe` at base through statement fallbacks 10s and 10z, a non-table
//! statement route, and the condition and return readers. Each now refuses
//! with the rule 11v (6) refusal. Every kinded use stays admitted, with its
//! place: `Safe` before the free and `use_after_close` after it.
//!
//! `shapeAdmissions` in `proof/QCore/InPlaceMentionGuard.lean` is bound here
//! row for row. The fixtures use the generic `k_*` surface; a `Safe` here is
//! over synthetic trusted test assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts, UnrestrictedLegacyResult,
    analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const LEAN: &str = include_str!("../../../proof/QCore/InPlaceMentionGuard.lean");
const KINDS: &str = include_str!("fixtures/contracts/resource_kinds_generic.qv");

/// Two unkinded callees of the census: a borrowed address and a by-value
/// scalar.
const UNKINDED: &str = "module contract;
extern fn k_peek(p: ptr<const, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
};
extern fn k_take(n: usize) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};
";

const DECLARATIONS: &str = "\
typedef struct { void *buffer; unsigned long used; } regex_t; \
void k_free(void *p); \
int k_regcomp(regex_t *preg, const char *pattern, int flags); \
int k_regexec(const regex_t *preg, const char *text); \
void k_regfree(regex_t *preg); \
int k_peek(const void *p); int k_take(__SIZE_TYPE__ n);\n";

const PREFIX: &str = "int w(const char *p, const char *s) { regex_t re; int r; int t; \
     unsigned long n; r = k_regcomp(&re, p, 0); if (r) return 0; ";

const STRAY: &str = "tracked in-place object `re` named outside its declarator, a `sizeof` operand or a declared in-place position (rule 11v (6))";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("resource_kinds_generic.qcore".to_owned(), KINDS.to_owned()),
        ("unkinded.qcore".to_owned(), UNKINDED.to_owned()),
    ]))
    .expect("the fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent mention-guard test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Outcome {
    status: &'static str,
    unsupported: Vec<String>,
    use_after_close: bool,
}

impl Outcome {
    fn stray(&self) -> bool {
        self.unsupported.iter().any(|text| text == STRAY)
    }
}

fn run(body: &str) -> Outcome {
    let source = format!("{DECLARATIONS}{body}\n");
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
    let summary = match &result {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    let diagnostics = summary
        .and_then(|summary| summary.functions.get("w"))
        .map(|function| function.diagnostics.clone())
        .unwrap_or_default();
    Outcome {
        status,
        unsupported: diagnostics
            .iter()
            .filter_map(|diagnostic| match &diagnostic.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect(),
        use_after_close: diagnostics.iter().any(|diagnostic| {
            matches!(diagnostic.kind, LegacyDiagnosticKind::UseAfterClose { .. })
        }),
    }
}

fn pre(statement: &str) -> Outcome {
    run(&format!("{PREFIX}{statement} k_regfree(&re); return 0; }}"))
}

fn post(statement: &str) -> Outcome {
    run(&format!("{PREFIX}k_regfree(&re); {statement} return 0; }}"))
}

/// The census escapes: each was certified `Safe` at base on both runs (the
/// mixed statement and the mixed condition on `pre` only), and each now
/// refuses with the rule 11v (6) refusal on both.
#[test]
fn every_census_escape_is_refused() {
    if !clang_available() {
        return;
    }
    for statement in [
        // Statement fallback 10s (the cast route).
        "(void) k_take(re.used);",
        "(void) k_take((unsigned long) re.used);",
        "(void) re.used;",
        "(void) (n = re.used);",
        "(void) (re.used = 0);",
        // Statement fallback 10z.
        "(void) (re.used ? 1 : 0);",
        // A non-table statement route.
        "t = k_regexec(&re, s) + (int) re.used;",
        // The condition readers.
        "if (re.used) t = 1;",
        "while (re.used) { n = 0; break; }",
        "do { n = 0; } while (re.used);",
        "switch (re.used) { default: break; }",
        "if ((n = re.used)) t = 1;",
        "if (k_regexec(&re, s) == 0 && re.used) t = 1;",
    ] {
        for (run, outcome) in [("pre", pre(statement)), ("post", post(statement))] {
            assert_ne!(outcome.status, "Safe", "{run}: {statement}: {outcome:?}");
            assert!(outcome.stray(), "{run}: {statement}: {outcome:?}");
        }
    }
    // The return reader.
    let outcome = run(&format!("{PREFIX}k_regfree(&re); return (int) re.used; }}"));
    assert_eq!(outcome.status, "Unknown", "{outcome:?}");
    assert!(outcome.stray(), "{outcome:?}");
}

/// A shape the guard already refused keeps its refusal; the rule 11v (6)
/// refusal is added beside it.
#[test]
fn a_guarded_refusal_stays_a_refusal() {
    if !clang_available() {
        return;
    }
    for statement in ["n = re.used;", "re.used = 0;", "(void) re;"] {
        let outcome = post(statement);
        assert_eq!(outcome.status, "Unknown", "{statement}: {outcome:?}");
        assert!(outcome.stray(), "{statement}: {outcome:?}");
        assert!(
            outcome
                .unsupported
                .iter()
                .any(|text| text.starts_with("tracked in-place object `re` used outside")),
            "{statement}: {outcome:?}"
        );
    }
}

/// Every kinded use is admitted with its place, on every route that lowers
/// it: `Safe` before the free, `use_after_close` after it.
#[test]
fn every_kinded_use_stays_admitted_with_its_place() {
    if !clang_available() {
        return;
    }
    for statement in [
        "k_regexec(&re, s);",
        "t = k_regexec(&re, s);",
        "t = k_regexec(&re, s) != 1;",
        "t = (int) k_regexec(&re, s);",
        "t = ({ k_regexec(&re, s); });",
        "if (k_regexec(&re, s) == 0) t = 1;",
        "int d = k_regexec(&re, s);",
    ] {
        let before = pre(statement);
        assert_eq!(before.status, "Safe", "pre: {statement}: {before:?}");
        let after = post(statement);
        assert_eq!(after.status, "Unsafe", "post: {statement}: {after:?}");
        assert!(after.use_after_close, "post: {statement}: {after:?}");
        assert!(!after.stray(), "post: {statement}: {after:?}");
    }
    let returned = run(&format!(
        "{PREFIX}k_regfree(&re); return k_regexec(&re, s) != 1; }}"
    ));
    assert_eq!(returned.status, "Unsafe", "{returned:?}");
    assert!(returned.use_after_close, "{returned:?}");
}

/// The `rd_regex_match` shape, verbatim but for the generic surface, and the
/// census mutant that reads a member after the free.
#[test]
fn the_regex_match_shape_is_safe_and_its_mutant_is_refused() {
    if !clang_available() {
        return;
    }
    let body = "int w(const char *pattern, const char *str) { regex_t re; int r; \
         r = k_regcomp(&re, pattern, 1 | 8); if (r) { return 0; } \
         r = k_regexec(&re, str) != 1; k_regfree(&re); return r; }";
    let outcome = run(body);
    assert_eq!(outcome.status, "Safe", "{outcome:?}");
    let mutant = run(&body.replace("k_regfree(&re);", "k_regfree(&re); (void) re.used;"));
    assert_eq!(mutant.status, "Unknown", "{mutant:?}");
    assert!(mutant.stray(), "{mutant:?}");
}

/// `sizeof`'s operand is not evaluated: it stays admitted.
#[test]
fn an_unevaluated_operand_stays_admitted() {
    if !clang_available() {
        return;
    }
    for statement in ["n = sizeof re;", "n = sizeof(re);"] {
        let outcome = post(statement);
        assert_eq!(outcome.status, "Safe", "{statement}: {outcome:?}");
    }
}

/// A body with no tracked object is untouched: an unkinded address of a
/// plain local is the formal-address gate's, as before.
#[test]
fn a_body_with_no_tracked_object_is_untouched() {
    if !clang_available() {
        return;
    }
    let outcome = run("int w(void) { unsigned long n = 0; (void) k_take(n); return 0; }");
    assert_eq!(outcome.status, "Safe", "{outcome:?}");
    assert!(!outcome.stray(), "{outcome:?}");
}

/// The rows of a Lean `(String × Bool)` list opening at `header`.
fn lean_admissions(header: &str) -> Vec<(String, bool)> {
    let start = LEAN.find(header).expect("the Lean list exists") + header.len();
    let body = &LEAN[start..];
    let end = body.find("\n]").expect("the Lean list closes");
    body[..end]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("(\""))
        .map(|line| {
            let line = line.trim_end_matches(',').trim_end_matches(')');
            let (shape, admitted) = line.rsplit_once(", ").expect("a pair");
            (
                shape
                    .trim_start_matches("(\"")
                    .trim_end_matches('"')
                    .to_owned(),
                admitted == "true",
            )
        })
        .collect()
}

/// `shapeAdmissions`, row for row: an admitted shape is `Safe` before the
/// free and carries no rule 11v (6) refusal; any other carries it.
#[test]
fn the_lean_shape_admissions_match_the_gateway() {
    if !clang_available() {
        return;
    }
    let rows = lean_admissions("def shapeAdmissions : List (String × Bool) := [");
    assert_eq!(rows.len(), 13);
    for (shape, admitted) in rows {
        let outcome = if shape.starts_with("return ") {
            run(&format!("{PREFIX}k_regfree(&re); {shape} }}"))
        } else {
            pre(&shape)
        };
        assert_eq!(!outcome.stray(), admitted, "{shape}: {outcome:?}");
        if admitted {
            assert_eq!(outcome.status, "Safe", "{shape}: {outcome:?}");
        } else {
            assert_ne!(outcome.status, "Safe", "{shape}: {outcome:?}");
        }
    }
}
