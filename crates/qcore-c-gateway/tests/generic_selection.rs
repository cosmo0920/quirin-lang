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

//! Card W44-C11-GENERIC-SELECTION — C11 generic selections.
//!
//! glibc 2.43's C23 const-preserving string macros expand a call such as
//! `strchr (S, C)` to
//!
//! ```text
//! _Generic (0 ? (S) : (void *) 1, const void *: (const char *) (strchr (S, C)),
//!           default: strchr (S, C))
//! ```
//!
//! The fixtures reproduce that macro over the generic `d_*` surface
//! (`fixtures/contracts/derived_result_generic.qcore`), so `d_chr` below is
//! spelled exactly as a `_GNU_SOURCE` unit spells `strchr`. `Safe` here is
//! `Safe` over synthetic trusted test assumptions and promotes nothing.
//!
//! The central property is that an admitted selection is analysed **exactly as
//! the call it selects**: every body is analysed twice, once through the macro
//! and once with the bare call written out, and the two complete diagnostic
//! vectors and statuses must be equal, in every position (a statement, a
//! declaration, a condition, a return, a call argument, nested). The refusing
//! cases pin that a selection that is not selection-invariant keeps its
//! refusal.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = include_str!("fixtures/contracts/derived_result_generic.qv");

/// The fixture contracts' C declarations, then glibc's macro over `d_chr`.
/// The declarations precede the macro, as in `string.h`, so they are not
/// themselves expanded.
const PRELUDE: &str = "\
char *d_alloc(__SIZE_TYPE__ n);
void d_free(void *p);
void d_consume(void *p);
__SIZE_TYPE__ d_len(const char *s);
char *d_chr(const char *s, int c);
#define q_const_generic(PTR, CTYPE, CALL) \\
  _Generic (0 ? (PTR) : (void *) 1, const void *: (CTYPE) (CALL), default: CALL)
#define d_chr(S, C) q_const_generic (S, const char *, d_chr (S, C))
";

/// The same declarations with no macro: the call as a pre-C23 header spells it.
const PLAIN: &str = "\
char *d_alloc(__SIZE_TYPE__ n);
void d_free(void *p);
void d_consume(void *p);
__SIZE_TYPE__ d_len(const char *s);
char *d_chr(const char *s, int c);
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "derived_result_generic.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the derived-result fixture loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent generic-selection test: {reason}");
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

/// The status and the complete, sorted, de-duplicated diagnostic vector of `w`.
fn outcome(prelude: &str, body: &str) -> (&'static str, Vec<String>) {
    let source = format!("{prelude}{body}\n");
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
    let function = program(&result)
        .functions
        .get("w")
        .expect("missing function `w`");
    let mut keys = function
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
            LegacyDiagnosticKind::DoubleRelease { .. } => "double_release".to_owned(),
            LegacyDiagnosticKind::ReleaseAfterEscape { .. } => "release_after_escape".to_owned(),
            LegacyDiagnosticKind::CustodyConflict { .. } => "custody_conflict".to_owned(),
            LegacyDiagnosticKind::InvalidRelease { .. } => "invalid_release".to_owned(),
            LegacyDiagnosticKind::UseAfterRelease { .. } => "use_after_release".to_owned(),
            LegacyDiagnosticKind::KindMismatchedRelease { .. } => "invalid_release".to_owned(),
            LegacyDiagnosticKind::UseAfterClose { .. } => "use_after_release".to_owned(),
            LegacyDiagnosticKind::CountedUser { .. } => "counted_user".to_owned(),
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            LegacyDiagnosticKind::SummaryConflict { .. } => "summary_conflict".to_owned(),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
}

/// `body` through glibc's macro is analysed exactly as the bare call, and that
/// outcome is the pinned one.
fn same_as_the_call(label: &str, body: &str, status: &str, vector: &[&str]) {
    let through_macro = outcome(PRELUDE, body);
    let spelled = outcome(PLAIN, body);
    assert_eq!(
        through_macro, spelled,
        "{label}: the selection is not analysed as its call\n{body}"
    );
    assert_eq!(
        (
            through_macro.0,
            through_macro
                .1
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        ),
        (status, vector.to_vec()),
        "{label}\n{body}"
    );
}

// -- The glibc shape, in its two selections -----------------------------------

#[test]
fn a_const_argument_selects_the_cast_and_is_analysed_as_the_call() {
    if !clang_available() {
        return;
    }
    same_as_the_call(
        "strchr over a caller's const string, null-tested, then read",
        "void w(const char *s) { const char *p; p = d_chr(s, 58); if (p) { d_len(p); } }",
        "Safe",
        &[],
    );
}

#[test]
fn a_mutable_argument_selects_the_call_and_keeps_its_derivation() {
    if !clang_available() {
        return;
    }
    // The derived result must survive the selection: releasing the base still
    // invalidates it, exactly as for the bare call.
    same_as_the_call(
        "strchr, free(base), strlen(p)",
        "void w(void) { char *b; char *p; b = d_alloc(8); p = d_chr(b, 58); \
         d_free(b); d_len(p); }",
        "Unsafe",
        &["use_after_release"],
    );
    // And the caller still may not release the interior pointer.
    same_as_the_call(
        "free(strchr(...)) then free(base)",
        "void w(void) { char *b; char *p; b = d_alloc(8); p = d_chr(b, 58); \
         d_free(p); d_free(b); }",
        "Unsafe",
        &["invalid_release"],
    );
}

// -- Every position ------------------------------------------------------------

#[test]
fn a_declaration_initializer_is_analysed_as_the_call() {
    if !clang_available() {
        return;
    }
    same_as_the_call(
        "const char *p = strchr(s, c);",
        "void w(const char *s) { const char *p = d_chr(s, 58); if (p) { d_len(p); } }",
        "Safe",
        &[],
    );
}

#[test]
fn a_condition_is_analysed_as_the_call() {
    if !clang_available() {
        return;
    }
    same_as_the_call(
        "if (strchr(s, c))",
        "void w(const char *s) { if (d_chr(s, 58)) { return; } }",
        "Safe",
        &[],
    );
    same_as_the_call(
        "if (!strchr(s, c))",
        "void w(char *s) { if (!d_chr(s, 58)) { return; } d_len(s); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_return_is_analysed_as_the_call() {
    if !clang_available() {
        return;
    }
    let spelled = outcome(
        PLAIN,
        "const char *w(const char *s) { return d_chr(s, 58); }",
    );
    same_as_the_call(
        "return strchr(s, c);",
        "const char *w(const char *s) { return d_chr(s, 58); }",
        spelled.0,
        &spelled.1.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    assert!(!spelled.1.iter().any(|key| key.contains("_Generic")));
}

#[test]
fn a_call_argument_is_analysed_as_the_call() {
    if !clang_available() {
        return;
    }
    let body = "void w(const char *s) { d_len(d_chr(s, 58)); }";
    let spelled = outcome(PLAIN, body);
    same_as_the_call(
        "strlen(strchr(s, c))",
        body,
        spelled.0,
        &spelled.1.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    assert!(!spelled.1.iter().any(|key| key.contains("_Generic")));
}

#[test]
fn a_nested_selection_is_analysed_as_the_call() {
    if !clang_available() {
        return;
    }
    // `d_chr (d_chr (s, 58), 59)` nests one selection in each association and
    // in the controlling expression of the other.
    let body = "void w(const char *s) { const char *p; p = d_chr(s, 58); \
                if (p) { const char *q; q = d_chr(d_chr(p, 59), 60); if (q) { d_len(q); } } }";
    let spelled = outcome(PLAIN, body);
    same_as_the_call(
        "strchr(strchr(s, c), d)",
        body,
        spelled.0,
        &spelled.1.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    assert!(!spelled.1.iter().any(|key| key.contains("_Generic")));
    // A selection nested directly in a selection's associations.
    let nested = "void w(const char *s) { const char *p; \
                  p = _Generic(s, const void *: (const char *) (d_chr(s, 58)), \
                  default: d_chr(s, 58)); if (p) { d_len(p); } }";
    assert_eq!(outcome(PRELUDE, nested), ("Safe", Vec::new()));
}

// -- The controlling expression is not evaluated -------------------------------

#[test]
fn a_controlling_expression_with_a_side_effect_contributes_nothing() {
    if !clang_available() {
        return;
    }
    // Were `d_alloc(8)` evaluated, it would leak on every path. C does not
    // evaluate it, and neither does the lowering: the body certifies.
    let body = "void w(const char *s) { const char *p; \
                p = _Generic(d_alloc(8), char *: d_chr(s, 58), default: d_chr(s, 58)); \
                if (p) { d_len(p); } }";
    assert_eq!(outcome(PLAIN, body), ("Safe", Vec::new()));
    // The same acquisition, evaluated, is a leak.
    let evaluated = "void w(const char *s) { const char *p; char *b; b = d_alloc(8); \
                     p = d_chr(s, 58); if (p) { d_len(p); } }";
    assert_eq!(outcome(PLAIN, evaluated).1, vec!["leak".to_owned()]);
}

// -- Clang's selection (card CG07-GENERIC-AND-STATEMENT-EXPRESSIONS) ------------
//
// The selections below were refused by route 1 (the associations differ, or a
// removed cast cannot be justified) and pinned as `call to uncontracted
// function \`_Generic\``. **Flipped by CG07 (rule 11g):** route 2 now lowers
// each to the association Clang selected, so each is analysed exactly as that
// association spelled directly, and no refusal names the selection.

/// `body` is analysed exactly as `spelled` (the selected association written
/// out), and nothing names `_Generic`.
fn as_selected(label: &str, body: &str, spelled: &str) {
    let selected = outcome(PLAIN, body);
    assert_eq!(selected, outcome(PLAIN, spelled), "{label}\n{body}");
    assert!(
        !selected.1.iter().any(|key| key.contains("_Generic")),
        "{label}: {selected:?}"
    );
}

#[test]
fn associations_that_differ_read_clangs_selection() {
    if !clang_available() {
        return;
    }
    as_selected(
        "two different calls",
        "void w(const char *s) { const char *p; \
         p = _Generic(s, const char *: d_chr(s, 58), default: d_chr(s, 59)); \
         if (p) { d_len(p); } }",
        "void w(const char *s) { const char *p; p = d_chr(s, 58); if (p) { d_len(p); } }",
    );
    as_selected(
        "a call and a name",
        "void w(const char *s) { const char *p; \
         p = _Generic(s, const char *: d_chr(s, 58), default: s); \
         if (p) { d_len(p); } }",
        "void w(const char *s) { const char *p; p = d_chr(s, 58); if (p) { d_len(p); } }",
    );
    // The default selected: the other association's call is not evaluated.
    as_selected(
        "the default association",
        "void w(const char *s) { const char *p; \
         p = _Generic(s, int: d_chr(s, 58), default: s); \
         if (p) { d_len(p); } }",
        "void w(const char *s) { const char *p; p = s; if (p) { d_len(p); } }",
    );
}

#[test]
fn a_cast_of_an_integer_result_reads_clangs_selection() {
    if !clang_available() {
        return;
    }
    // `s` is `const char *`: no association names its type, so Clang selects
    // the default, `d_len (s)`; the cast of the other is never evaluated.
    as_selected(
        "a pointer cast of an integer call",
        "void w(const char *s) { \
         _Generic(s, const void *: (const char *) (d_len(s)), default: d_len(s)); }",
        "void w(const char *s) { d_len(s); }",
    );
}

#[test]
fn a_cast_that_does_not_cover_the_association_reads_clangs_selection() {
    if !clang_available() {
        return;
    }
    // Clang selects the default, `d_chr (s, 58) + 1`, kept parenthesised.
    as_selected(
        "a cast of a prefix",
        "void w(const char *s) { const char *p; \
         p = _Generic(s, const void *: (const char *) d_chr(s, 58) + 1, \
         default: d_chr(s, 58) + 1); if (p) { d_len(p); } }",
        "void w(const char *s) { const char *p; p = (d_chr(s, 58) + 1); if (p) { d_len(p); } }",
    );
}
