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

use super::super::{function_names, tokenize_with_conditionals};
use super::{derivation_scope, has_internal_linkage};

fn scope(source: &str, selected: &[&str]) -> Vec<String> {
    let (tokens, conditionals) = tokenize_with_conditionals(source);
    let definitions = function_names(&tokens, &conditionals).definitions;
    let selected = selected
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    let mut scope = derivation_scope(&tokens, &selected, &definitions, true);
    scope.sort();
    scope
}

fn linkage(source: &str, function: &str) -> bool {
    let (tokens, _) = tokenize_with_conditionals(source);
    has_internal_linkage(&tokens, function)
}

#[test]
fn a_static_definition_has_internal_linkage() {
    assert!(linkage("static void f(void *p) { }", "f"));
    assert!(linkage("static inline void f(void *p) { }", "f"));
    assert!(linkage(
        "int x; static __attribute__((unused)) inline int f(int a) { return a; }",
        "f"
    ));
    assert!(linkage(
        "void g(void) { } static void *f(void) { return 0; }",
        "f"
    ));
}

#[test]
fn a_definition_without_static_is_not_established() {
    assert!(!linkage("void f(void *p) { }", "f"));
    assert!(!linkage("inline void f(void *p) { }", "f"));
    assert!(!linkage("extern inline void f(void *p) { }", "f"));
    // `static` on an earlier declaration alone: internal by C11 6.2.2p5, but
    // the reader reads the definition only and answers "not established".
    assert!(!linkage("static void f(void *p); void f(void *p) { }", "f"));
    // The previous definition's `static` does not leak across its `}`.
    assert!(!linkage("static void g(void) { } void f(void) { }", "f"));
    // No definition at all.
    assert!(!linkage("static void f(void *p);", "f"));
}

#[test]
fn the_scope_adds_reached_internal_definitions_only() {
    let source = "static void helper(void *p) { inner(p); }\n\
                  static void inner(void *p) { }\n\
                  static void unreached(void *p) { }\n\
                  inline void external(void *p) { }\n\
                  void selected(void *p) { helper(p); external(p); }\n";
    assert_eq!(
        scope(source, &["selected"]),
        vec!["helper", "inner", "selected"]
    );
}

#[test]
fn without_a_selection_the_scope_is_unchanged() {
    let source = "static void helper(void *p) { }\nvoid selected(void *p) { helper(p); }\n";
    let (tokens, conditionals) = tokenize_with_conditionals(source);
    let definitions = function_names(&tokens, &conditionals).definitions;
    assert_eq!(
        derivation_scope(&tokens, &definitions, &definitions, false),
        definitions
    );
}

#[test]
fn a_selected_static_definition_is_not_duplicated() {
    let source = "static void helper(void *p) { }\nstatic void selected(void *p) { helper(p); }\n";
    assert_eq!(
        scope(source, &["selected", "helper"]),
        vec!["helper", "selected"]
    );
}

/// Re-pinned by card W44-LIBC-RESIDUALS, which lifted the reserved-name
/// filter: a reserved static definition the unit carries is admitted like any
/// other, and a reserved name the unit only declares is not.
#[test]
fn a_reserved_static_definition_is_admitted_only_where_the_unit_carries_it() {
    let source = "static __inline unsigned int __bswap_32(unsigned int x) { return x; }\n\
                  static inline int _Upper(int x) { return x; }\n\
                  static inline int _lower(int x) { return x; }\n\
                  static int __declared_only(int x);\n\
                  extern inline int __external_inline(int x) { return x; }\n\
                  void selected(unsigned int x) { __bswap_32(x); _Upper(1); _lower(2); \
                  __declared_only(3); __external_inline(4); }\n";
    assert_eq!(
        scope(source, &["selected"]),
        vec!["_Upper", "__bswap_32", "_lower", "selected"]
    );
}

// --- End to end, through the selected-unit analysis -------------------------

use super::super::{
    AnalysisStatus, LegacyDiagnosticKind, LegacyProgramSummary,
    analyze_legacy_c_translation_unit_selected, analyze_legacy_c_translation_unit_selected_partial,
};
use qcore_schemas::contract::ResourceContracts;

const Q_BASE: (&str, &str) = (
    "q_base.qcore",
    include_str!("../fixtures/contracts/q_base.qv"),
);

fn contracts() -> ResourceContracts {
    let sources = [(Q_BASE.0.to_owned(), Q_BASE.1.to_owned())]
        .into_iter()
        .collect();
    crate::contract_loader::load_resource_contract_sources(&sources).unwrap()
}

/// `subject` alone is selected, as a main-source enumeration would select it
/// when the helper arrives through a header.
fn selected(source: &str) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit_selected(
        source,
        "subject",
        &contracts(),
        Some(&["subject".to_owned()]),
    )
    .unwrap()
}

fn uncontracted(summary: &LegacyProgramSummary, callee: &str) -> bool {
    summary.functions["subject"]
        .diagnostics
        .iter()
        .any(|diagnostic| {
            matches!(&diagnostic.kind, LegacyDiagnosticKind::Unsupported { description }
                if description == &format!("call to uncontracted function `{callee}`"))
        })
}

const STATIC_RELEASE: &str = "static void release(void *p) { q_free(p); }\n\
     void subject(void) { void *p = q_alloc(); release(p); }\n";

#[test]
fn a_static_header_helper_is_summarised_and_its_caller_certifies() {
    let summary = selected(STATIC_RELEASE);
    // Only the selection is reported.
    assert_eq!(
        summary.functions.keys().cloned().collect::<Vec<_>>(),
        vec!["subject".to_owned()]
    );
    assert_eq!(summary.functions["subject"].status, AnalysisStatus::Safe);
    assert_eq!(summary.status, AnalysisStatus::Safe);
    // The helper's validated summary is published with the unit's records, so
    // a certificate check re-derives it rather than taking it on trust.
    let partial = analyze_legacy_c_translation_unit_selected_partial(
        STATIC_RELEASE,
        "subject",
        &contracts(),
        Some(&["subject".to_owned()]),
        (crate::legacy_c::CCompilerDialect::C11, None),
    )
    .unwrap();
    assert!(
        partial
            .summaries
            .iter()
            .any(|record| record.function == "release")
    );
}

#[test]
fn an_inferred_summary_is_never_weaker_than_the_body() {
    // The helper releases nothing, so the caller's acquisition is a leak: the
    // summary is `Borrowed`, and applying it exposes the leak (accusing).
    let summary = selected(
        "static void keep(void *p) { }\n\
         void subject(void) { void *p = q_alloc(); keep(p); }\n",
    );
    assert_eq!(summary.functions["subject"].status, AnalysisStatus::Unsafe);
    assert!(
        summary.functions["subject"]
            .diagnostics
            .iter()
            .any(|diagnostic| { matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. }) })
    );
}

#[test]
fn a_helper_whose_body_does_not_validate_leaves_the_call_uncontracted() {
    let summary = selected(
        "static void release_twice(void *p) { q_free(p); q_free(p); }\n\
         void subject(void) { void *p = q_alloc(); release_twice(p); }\n",
    );
    assert_ne!(summary.functions["subject"].status, AnalysisStatus::Safe);
    assert!(uncontracted(&summary, "release_twice"));
}

#[test]
fn an_external_inline_helper_is_refused_on_linkage() {
    // C11 6.7.4p7: the call may run the external definition instead.
    let summary = selected(
        "inline void release(void *p) { q_free(p); }\n\
         void subject(void) { void *p = q_alloc(); release(p); }\n",
    );
    assert_ne!(summary.functions["subject"].status, AnalysisStatus::Safe);
    assert!(uncontracted(&summary, "release"));
}

/// Re-pinned by card W44-LIBC-RESIDUALS: a reserved-name static helper the
/// unit carries is checked like any other, so its validated `Released` summary
/// discharges the acquisition; with no body, the call stays uncontracted.
#[test]
fn a_reserved_helper_is_checked_where_the_unit_carries_it() {
    let summary = selected(
        "static void __release(void *p) { q_free(p); }\n\
         void subject(void) { void *p = q_alloc(); __release(p); }\n",
    );
    assert_eq!(summary.functions["subject"].status, AnalysisStatus::Safe);
    assert!(!uncontracted(&summary, "__release"));
    let summary = selected(
        "static void __release(void *p);\n\
         void subject(void) { void *p = q_alloc(); __release(p); }\n",
    );
    assert_ne!(summary.functions["subject"].status, AnalysisStatus::Safe);
    assert!(uncontracted(&summary, "__release"));
}

#[test]
fn a_contradicting_static_definition_withdraws_the_declaration() {
    // `q_free` is declared `Released`; a reached static definition of the same
    // name releases nothing. The derivation now compares it, and the
    // declared-mode gate withdraws the declaration at the call site
    // (the LOADER-ABI-MODES-SELECTION consequence, refusing direction).
    let summary = selected(
        "static void q_free(void *p) { }\n\
         void subject(void) { void *p = q_alloc(); q_free(p); }\n",
    );
    assert_ne!(summary.functions["subject"].status, AnalysisStatus::Safe);
    assert!(
        summary.functions["subject"]
            .diagnostics
            .iter()
            .any(|diagnostic| {
                matches!(&diagnostic.kind, LegacyDiagnosticKind::Unsupported { description }
            if description.contains("mode-parameter-contradiction"))
            })
    );
}

#[test]
fn without_a_selection_the_helper_is_a_reported_row_as_before() {
    let summary =
        analyze_legacy_c_translation_unit_selected(STATIC_RELEASE, "subject", &contracts(), None)
            .unwrap();
    assert_eq!(
        summary.functions.keys().cloned().collect::<Vec<_>>(),
        vec!["release".to_owned(), "subject".to_owned()]
    );
    assert_eq!(summary.status, AnalysisStatus::Safe);
}
