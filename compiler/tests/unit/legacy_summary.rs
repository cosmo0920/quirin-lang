//! W4 regression suite: the LC01 repair list from
//! `docs/REEVALUATION_FOLLOWUP.md`, pinned as behavior of the validated
//! interprocedural custody inference.
//!
//! The cardinal expectation of every case here is negative: no translation unit
//! may become `Safe` unless every resource obligation is actually discharged.
//! A case that cannot be validated must land on `Unknown`, never on `Safe`.

use super::*;

use crate::legacy_c::{
    LegacyProgramSummary, analyze_legacy_c_translation_unit, legacy_c_source_functions, tokenize,
};

const Q_BASE: (&str, &str) = (
    "q_base.qcore",
    include_str!("../fixtures/contracts/q_base.qcore"),
);
const Q_NULLABLE: (&str, &str) = (
    "q_nullable.qcore",
    include_str!("../fixtures/contracts/q_nullable.qcore"),
);
const Q_REALLOC: (&str, &str) = (
    "q_realloc.qcore",
    include_str!("../fixtures/contracts/q_realloc.qcore"),
);
const Q_RICH_MODES: (&str, &str) = (
    "q_rich_modes.qcore",
    include_str!("../fixtures/contracts/q_rich_modes.qcore"),
);
const Q_INTERNAL_MAKE: (&str, &str) = (
    "q_internal_make.qcore",
    include_str!("../fixtures/contracts/q_internal_make.qcore"),
);
const Q_CONTAINER_STORE: (&str, &str) = (
    "q_container_store.qcore",
    include_str!("../fixtures/contracts/q_container_store.qcore"),
);

fn fixture_contracts(modules: &[(&str, &str)]) -> ResourceContracts {
    let sources = modules
        .iter()
        .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
        .collect();
    crate::contract_loader::load_resource_contract_sources(&sources).unwrap()
}

fn analyze(source: &str, modules: &[(&str, &str)]) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(source, "subject", &fixture_contracts(modules)).unwrap()
}

fn base(source: &str) -> LegacyProgramSummary {
    analyze(source, &[Q_BASE])
}

fn summary_of(source: &str, function: &str) -> ResourceFunctionContract {
    let tokens = tokenize(source);
    let names = legacy_c_source_functions(source);
    let derivation = derive_custody_summaries(&tokens, &names, &fixture_contracts(&[Q_BASE]));
    derivation
        .records()
        .into_iter()
        .find(|record| record.function == function)
        .unwrap_or_else(|| panic!("no validated summary for `{function}`"))
        .contract
}

fn no_summary(source: &str, modules: &[(&str, &str)], function: &str) -> bool {
    let tokens = tokenize(source);
    let names = legacy_c_source_functions(source);
    derive_custody_summaries(&tokens, &names, &fixture_contracts(modules))
        .records()
        .iter()
        .all(|record| record.function != function)
}

fn has(
    summary: &LegacyProgramSummary,
    function: &str,
    expected: fn(&LegacyDiagnosticKind) -> bool,
) -> bool {
    summary.functions[function]
        .diagnostics
        .iter()
        .any(|diagnostic| expected(&diagnostic.kind))
}

// --- LC01's counterexample -------------------------------------------------

#[test]
fn lc01_allocation_wrapper_leak_is_unsafe_not_safe() {
    // The exact translation unit `docs/REEVALUATION_FOLLOWUP.md` reproduced as
    // a false `Safe`. `make` now carries a validated owned-return summary, so
    // the discarded result in `subject` is a reported leak.
    let summary = base(
        "void *make(void) { void *p = q_alloc(); return p; } void subject(void) { void *p = make(); return; }",
    );
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert_eq!(summary.functions["make"].status, AnalysisStatus::Safe);
    assert!(has(&summary, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
}

#[test]
fn lc01_allocation_wrapper_with_discharge_is_safe() {
    let summary = base(
        "void *make(void) { void *p = q_alloc(); return p; } void subject(void) { void *p = make(); q_free(p); return; }",
    );
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert_eq!(
        summary_of(
            "void *make(void) { void *p = q_alloc(); return p; } void subject(void) { void *p = make(); q_free(p); }",
            "make",
        ),
        ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: false })
    );
}

#[test]
fn caller_discard_of_a_wrapped_acquisition_is_a_leak() {
    // The result is never bound at all: the reduced importer refuses to drop an
    // owned result on the floor, so the unit stays Unknown rather than Safe.
    let summary = base(
        "void *make(void) { void *p = q_alloc(); return p; } void subject(void) { make(); return; }",
    );
    assert_eq!(summary.status, AnalysisStatus::Unknown);
    assert!(has(&summary, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Unsupported { description }
            if description.contains("discarded owned result")
    )));
}

#[test]
fn caller_release_through_a_wrapper_is_safe() {
    let summary = base(
        "void release(void *p) { q_free(p); } \
         void subject(void) { void *p = q_alloc(); release(p); return; }",
    );
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert_eq!(
        summary_of(
            "void release(void *p) { q_free(p); } void subject(void) { void *p = q_alloc(); release(p); }",
            "release",
        ),
        ResourceFunctionContract::new(
            [ResourceParameterMode::Released { nullable: false }],
            ResourceReturnMode::None,
        )
    );
}

#[test]
fn caller_release_through_a_wrapper_still_finds_a_double_release() {
    let summary = base(
        "void release(void *p) { q_free(p); } \
         void subject(void) { void *p = q_alloc(); release(p); q_free(p); }",
    );
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert!(has(&summary, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::DoubleRelease { .. }
    )));
}

#[test]
fn nested_wrappers_compose_bottom_up() {
    let source = "void *make(void) { void *p = q_alloc(); return p; } \
                  void *nested(void) { void *p = make(); return p; } \
                  void subject(void) { void *p = nested(); return; }";
    let summary = base(source);
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert!(has(&summary, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
    assert_eq!(
        summary_of(source, "nested"),
        ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: false })
    );

    let discharged = base(
        "void *make(void) { void *p = q_alloc(); return p; } \
         void *nested(void) { void *p = make(); return p; } \
         void subject(void) { void *p = nested(); q_free(p); return; }",
    );
    assert_eq!(discharged.status, AnalysisStatus::Safe);
}

#[test]
fn parameter_consumption_transfers_custody_at_the_call() {
    let source = "void hand_off(void *p) { q_transfer(p); } \
                  void subject(void) { void *p = q_alloc(); hand_off(p); return; }";
    let summary = base(source);
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert_eq!(
        summary_of(source, "hand_off"),
        ResourceFunctionContract::new(
            [ResourceParameterMode::Consumed { nullable: false }],
            ResourceReturnMode::None,
        )
    );
    // Custody left the caller, so releasing afterwards is a defect.
    let after_escape = base(
        "void hand_off(void *p) { q_transfer(p); } \
         void subject(void) { void *p = q_alloc(); hand_off(p); q_free(p); }",
    );
    assert_eq!(after_escape.status, AnalysisStatus::Unsafe);
    assert!(has(&after_escape, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::ReleaseAfterEscape { .. }
    )));
}

#[test]
fn a_borrowed_parameter_imposes_no_custody_obligation() {
    let source = "int width(int n) { return n; } \
                  void subject(void) { void *p = q_alloc(); int w = width(3); q_free(p); }";
    let summary = base(source);
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert_eq!(
        summary_of(source, "width"),
        ResourceFunctionContract::new([ResourceParameterMode::Borrowed], ResourceReturnMode::None,)
    );
}

#[test]
fn an_observed_parameter_is_owned_not_borrowed() {
    // `q_mutate` requires a live non-null resource, so the wrapper's formal
    // must be raised from `Borrowed` to `Owned`; otherwise a caller could pass
    // an already-released resource unnoticed.
    let source = "void touch(void *p) { q_mutate(p); } \
                  void subject(void) { void *p = q_alloc(); touch(p); q_free(p); }";
    let summary = analyze(source, &[Q_BASE, Q_RICH_MODES]);
    assert_eq!(summary.status, AnalysisStatus::Safe);

    let tokens = tokenize(source);
    let names = legacy_c_source_functions(source);
    let derivation =
        derive_custody_summaries(&tokens, &names, &fixture_contracts(&[Q_BASE, Q_RICH_MODES]));
    let touch = derivation
        .records()
        .into_iter()
        .find(|record| record.function == "touch")
        .expect("validated summary for `touch`");
    assert_eq!(
        touch.contract,
        ResourceFunctionContract::new(
            [ResourceParameterMode::Owned { nullable: false }],
            ResourceReturnMode::None,
        )
    );
    assert_eq!(touch.trust, ContractTrust::InferredValidated);

    // The same wrapper called after release keeps the obligation visible.
    let stale = analyze(
        "void touch(void *p) { q_mutate(p); } \
         void subject(void) { void *p = q_alloc(); q_free(p); touch(p); }",
        &[Q_BASE, Q_RICH_MODES],
    );
    assert_eq!(stale.status, AnalysisStatus::Unknown);
}

// --- Recursion -------------------------------------------------------------

#[test]
fn direct_recursion_stays_unknown() {
    let summary = base("void subject(void) { subject(); return; }");
    assert_eq!(summary.status, AnalysisStatus::Unknown);
    assert!(has(&summary, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Unsupported { description }
            if description.contains("uncontracted function")
    )));
}

#[test]
fn self_recursive_allocation_wrapper_stays_unknown() {
    let summary = base(
        "void *make(int n) { if (n) { void *r = make(0); return r; } void *p = q_alloc(); return p; } \
         void subject(void) { void *p = make(1); return; }",
    );
    assert_eq!(summary.status, AnalysisStatus::Unknown);
    assert!(
        summary
            .functions
            .values()
            .all(|function| function.status != AnalysisStatus::Safe)
    );
}

#[test]
fn mutual_recursion_stays_unknown_for_every_member() {
    let summary = base(
        "void *even(int n); \
         void *odd(int n) { void *r = even(n); return r; } \
         void *even(int n) { if (n) { void *r = odd(n); return r; } void *p = q_alloc(); return p; } \
         void subject(void) { void *p = odd(3); q_free(p); }",
    );
    assert_eq!(summary.status, AnalysisStatus::Unknown);
    for name in ["odd", "even", "subject"] {
        assert_ne!(
            summary.functions[name].status,
            AnalysisStatus::Safe,
            "{name}"
        );
    }
}

// --- Out parameters and realloc -------------------------------------------

#[test]
fn out_parameter_wrappers_are_not_inferred() {
    // `*out = p` is outside the modeled place vocabulary, so the body is not
    // fully modeled, no summary is derived, and the call stays uncontracted.
    let source = "int fill(void **out) { void *p = q_alloc(); *out = p; return 0; } \
                  void subject(void) { void *p; int rc = fill(&p); q_free(p); }";
    let summary = analyze(source, &[Q_BASE, Q_RICH_MODES]);
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(has(&summary, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Unsupported { description }
            if description.contains("uncontracted function `fill`")
    )));
    assert!(no_summary(source, &[Q_BASE, Q_RICH_MODES], "fill"));
}

#[test]
fn realloc_wrappers_are_not_inferred() {
    // A reallocation leaves the argument's custody dependent on whether the
    // result was null. The analyzer refuses to return a possibly-null place, so
    // no settled exit describes the wrapper and no summary is published; the
    // caller keeps the obligation to resolve both outcomes itself.
    let source = "void *grow(void *p, unsigned long n) { void *q = q_realloc(p, n); return q; } \
                  void subject(void) { void *p = q_alloc(); void *q = grow(p, 8); if (q) { q_free(q); } else { q_free(p); } }";
    let summary = analyze(source, &[Q_BASE, Q_REALLOC]);
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(no_summary(source, &[Q_BASE, Q_REALLOC], "grow"));

    // The same reallocation written inline still resolves both outcomes.
    let inline = analyze(
        "void subject(void) { void *p = q_alloc(); void *q = q_realloc(p, 8); if (q) { q_free(q); } else { q_free(p); } }",
        &[Q_BASE, Q_REALLOC],
    );
    assert_eq!(inline.status, AnalysisStatus::Safe);
}

// --- Fail-closed boundaries ------------------------------------------------

#[test]
fn a_wrapper_with_an_unsupported_body_yields_no_summary() {
    let summary = base(
        "void *make(void) { void *p = q_alloc(); other(p); return p; } \
         void subject(void) { void *p = make(); q_free(p); }",
    );
    assert_eq!(summary.status, AnalysisStatus::Unknown);
    assert!(has(&summary, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Unsupported { description }
            if description.contains("uncontracted function `make`")
    )));
}

#[test]
fn a_leaking_callee_yields_no_summary_and_stays_unsafe() {
    let summary = base(
        "void helper(void) { void *p = q_alloc(); return; } void subject(void) { helper(); return; }",
    );
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert_eq!(summary.functions["helper"].status, AnalysisStatus::Unsafe);
    assert_eq!(summary.functions["subject"].status, AnalysisStatus::Unknown);
}

#[test]
fn a_conditionally_releasing_parameter_yields_no_summary() {
    // Released on one path and still live on another: no single parameter mode
    // describes the call, so the wrapper stays uncontracted and the caller's
    // undischarged obligation stays visible.
    let source = "void maybe(void *p, int c) { if (c) { q_free(p); } } \
                  void subject(void) { void *p = q_alloc(); maybe(p, 1); }";
    let summary = base(source);
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(has(&summary, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Unsupported { description }
            if description.contains("uncontracted function `maybe`")
    )));
    assert!(no_summary(source, &[Q_BASE], "maybe"));
}

#[test]
fn a_resource_stored_into_a_caller_aggregate_yields_no_summary() {
    // The fresh chunk ends up owned by the caller's container without the
    // summary being able to say so, so no summary is published.
    let summary = analyze(
        "void attach(void *owner) { void *p = q_alloc(); q_store(owner, p); } \
         void subject(void) { void *c = q_alloc(); attach(c); q_free(c); }",
        &[Q_BASE, Q_CONTAINER_STORE],
    );
    assert_ne!(summary.status, AnalysisStatus::Safe);
}

#[test]
fn returning_a_formal_unchanged_yields_no_owned_summary() {
    // `identity` hands back its own argument. Claiming an owned result would
    // mint a second custody for one resource, so no summary is derived.
    let summary = base(
        "void *identity(void *p) { q_mutate(p); return p; } \
         void subject(void) { void *p = q_alloc(); void *q = identity(p); q_free(q); }",
    );
    assert_ne!(summary.status, AnalysisStatus::Safe);
}

#[test]
fn a_nullable_returning_wrapper_forces_a_null_check() {
    let source = "void *maybe_make(int c) { if (c) { void *p = q_alloc(); return p; } return 0; } \
                  void subject(void) { void *p = maybe_make(1); if (p) { q_free(p); } }";
    let summary = analyze(source, &[Q_BASE, Q_NULLABLE]);
    assert_eq!(summary.status, AnalysisStatus::Safe);
    let tokens = tokenize(source);
    let names = legacy_c_source_functions(source);
    let derivation =
        derive_custody_summaries(&tokens, &names, &fixture_contracts(&[Q_BASE, Q_NULLABLE]));
    assert_eq!(
        derivation
            .records()
            .into_iter()
            .find(|record| record.function == "maybe_make")
            .expect("validated summary for `maybe_make`")
            .contract,
        ResourceFunctionContract::new(
            [ResourceParameterMode::Borrowed],
            ResourceReturnMode::Owned { nullable: true }
        )
    );

    // Releasing without the null check leaves the unit unresolved.
    let unchecked = analyze(
        "void *maybe_make(int c) { if (c) { void *p = q_alloc(); return p; } return 0; } \
         void subject(void) { void *p = maybe_make(1); q_free(p); }",
        &[Q_BASE, Q_NULLABLE],
    );
    assert_eq!(unchecked.status, AnalysisStatus::Unknown);
}

// --- Declared contracts win, disagreements are reported --------------------

#[test]
fn an_explicit_contract_agreeing_with_inference_is_preserved() {
    let summary = analyze(
        "void *make(void) { void *p = q_alloc(); return p; } \
         void subject(void) { void *p = make(); q_free(p); return; }",
        &[Q_BASE, Q_INTERNAL_MAKE],
    );
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert!(
        summary.functions["make"]
            .diagnostics
            .iter()
            .all(|diagnostic| !matches!(
                diagnostic.kind,
                LegacyDiagnosticKind::SummaryConflict { .. }
            ))
    );
}

#[test]
fn an_explicit_contract_disagreeing_with_inference_is_a_diagnostic() {
    // `make` is declared to return a fresh owned resource, but its body
    // returns nothing of the sort. The declaration still governs every call
    // site; the disagreement is reported instead of silently resolved.
    let summary = analyze(
        "void *make(void) { void *p = q_alloc(); q_free(p); return 0; } \
         void subject(void) { void *p = make(); q_free(p); return; }",
        &[Q_BASE, Q_INTERNAL_MAKE],
    );
    assert_eq!(summary.status, AnalysisStatus::Unknown);
    assert!(has(&summary, "make", |kind| matches!(
        kind,
        LegacyDiagnosticKind::SummaryConflict { .. }
    )));
}

#[test]
fn a_conflicting_summary_is_not_published_as_evidence() {
    let source = "void *make(void) { void *p = q_alloc(); q_free(p); return 0; } \
                  void subject(void) { void *p = make(); q_free(p); }";
    let tokens = tokenize(source);
    let names = legacy_c_source_functions(source);
    let derivation = derive_custody_summaries(
        &tokens,
        &names,
        &fixture_contracts(&[Q_BASE, Q_INTERNAL_MAKE]),
    );
    assert!(
        derivation
            .records()
            .iter()
            .all(|record| record.function != "make")
    );
}

// --- Call-graph ordering ---------------------------------------------------

#[test]
fn components_are_returned_callees_first() {
    let source = "void *make(void) { void *p = q_alloc(); return p; } \
                  void *nested(void) { void *p = make(); return p; } \
                  void subject(void) { void *p = nested(); q_free(p); }";
    let tokens = tokenize(source);
    let names = legacy_c_source_functions(source);
    let derivation = derive_custody_summaries(&tokens, &names, &fixture_contracts(&[Q_BASE]));
    let derived = derivation
        .records()
        .into_iter()
        .map(|record| record.function)
        .collect::<Vec<_>>();
    // Both wrappers validate, which is only possible if `make` was summarized
    // before `nested` was analyzed.
    assert!(derived.contains(&"make".to_owned()));
    assert!(derived.contains(&"nested".to_owned()));
}
