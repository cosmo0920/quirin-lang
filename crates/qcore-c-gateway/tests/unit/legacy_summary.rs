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

#[test]
fn ranked_recursive_lower_environment_keeps_cross_unit_import() {
    use qcore_schemas::contract::CrossUnitImportIdentity;
    let identity = CrossUnitImportIdentity {
        raw_source_sha256: "a".repeat(64),
        preprocessed_source_sha256: "b".repeat(64),
        scope_evidence_sha256: "c".repeat(64),
        analysis_request_sha256: "d".repeat(64),
        environment_sha256: "e".repeat(64),
    };
    let mut environment = ResourceContracts::default();
    environment.set_inferred_imports(BTreeMap::from([(
        "external".to_owned(),
        ResourceFunctionContract::default(),
    )]));
    environment
        .set_cross_unit_import_identities(BTreeMap::from([("external".to_owned(), identity)]));
    environment.set_link_resolution_sha256(Some("f".repeat(64)));
    environment.insert_contract("recursive".to_owned(), ResourceFunctionContract::default());
    let lower = component_lower_environment(&["recursive".to_owned()], &environment);
    assert!(lower.functions().get("recursive").is_none());
    assert_eq!(lower.inferred_imports(), environment.inferred_imports());
    assert_eq!(
        lower.cross_unit_import_identities(),
        environment.cross_unit_import_identities()
    );
    assert_eq!(
        lower.link_resolution_sha256(),
        environment.link_resolution_sha256()
    );
}

const Q_BASE: (&str, &str) = (
    "q_base.qcore",
    include_str!("../fixtures/contracts/q_base.qv"),
);
const Q_NULLABLE: (&str, &str) = (
    "q_nullable.qcore",
    include_str!("../fixtures/contracts/q_nullable.qv"),
);
const Q_REALLOC: (&str, &str) = (
    "q_realloc.qcore",
    include_str!("../fixtures/contracts/q_realloc.qv"),
);
const Q_RICH_MODES: (&str, &str) = (
    "q_rich_modes.qcore",
    include_str!("../fixtures/contracts/q_rich_modes.qv"),
);
const Q_INTERNAL_MAKE: (&str, &str) = (
    "q_internal_make.qcore",
    include_str!("../fixtures/contracts/q_internal_make.qv"),
);
const Q_CONTAINER_STORE: (&str, &str) = (
    "q_container_store.qcore",
    include_str!("../fixtures/contracts/q_container_store.qv"),
);
const Q_CONTAINER_REMOVE: (&str, &str) = (
    "q_container_remove.qcore",
    include_str!("../fixtures/contracts/q_container_remove.qv"),
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

fn summary_in(source: &str, modules: &[(&str, &str)], function: &str) -> ResourceFunctionContract {
    let tokens = tokenize(source);
    let names = legacy_c_source_functions(source);
    let record = derive_custody_summaries(&tokens, &names, &fixture_contracts(modules))
        .records()
        .into_iter()
        .find(|record| record.function == function)
        .unwrap_or_else(|| panic!("no validated summary for `{function}`"));
    assert_eq!(record.trust, ContractTrust::InferredValidated);
    record.contract
}

fn summary_of(source: &str, function: &str) -> ResourceFunctionContract {
    summary_in(source, &[Q_BASE], function)
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
    // Pin deliberately narrowed by card C03-NFR. Direct recursion by itself no
    // longer defeats inference: an *effect-free* recursive component now
    // publishes the neutral contract through the neutral-frame route, which is
    // pinned in `crates/qcore-c-gateway/tests/neutral_frame_recursion.rs`
    // together with the reason that start is admissible only at the neutral
    // element. What still stays `Unknown` — and is the load-bearing half of the
    // original expectation — is a recursive body that carries a custody effect:
    // the neutral candidate is refused, and no candidate above it is ever
    // seeded for a member of its own component.
    let summary = base(
        "void helper(void *p) { q_free(p); helper(p); } \
         void subject(void) { void *p = q_alloc(); helper(p); q_free(p); }",
    );
    assert_eq!(summary.status, AnalysisStatus::Unknown);
    for function in ["helper", "subject"] {
        assert!(
            has(&summary, function, |kind| matches!(
                kind,
                LegacyDiagnosticKind::Unsupported { description }
                    if description.contains("uncontracted function")
            )),
            "{function}"
        );
    }
}

#[test]
fn context_decreasing_allocation_wrapper_preserves_the_callers_leak() {
    let source = "void *make(int n) { if (n) { void *r = make(0); return r; } void *p = q_alloc(); return p; } \
         void subject(void) { void *p = make(1); return; }";
    let summary = base(source);
    assert_eq!(summary.functions["make"].status, AnalysisStatus::Safe);
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert!(has(&summary, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
    assert_eq!(
        summary_in(source, &[Q_BASE], "make").result,
        ResourceReturnMode::Owned { nullable: false }
    );
}

#[test]
fn a_self_consistent_recursive_summary_is_never_accepted() {
    // `bad` is consistent with "releases `p`" — but only for executions that
    // never terminate. There is no validated decreasing context edge to justify
    // that call; an assumed release would erase an obligation without evidence,
    // so no route may ever seed one.
    //
    // Pin sharpened by card C03-NFR. The neutral-frame route does publish a
    // summary for this component now, because `bad` performs no custody event
    // at all — but the summary it publishes is the *neutral* one, the bottom
    // element, which asserts the absence of an effect and can never assert one.
    // The load-bearing expectation is unchanged and now checked directly: the
    // self-consistent `Released` reading is still never accepted, and the
    // caller's acquisition is still an undischarged obligation.
    let source = "void bad(void *p) { bad(p); } \
                  void subject(void) { void *p = q_alloc(); bad(p); }";
    let summary = base(source);
    assert_eq!(
        summary_in(source, &[Q_BASE], "bad"),
        ResourceFunctionContract::new([ResourceParameterMode::Borrowed], ResourceReturnMode::None)
    );
    assert_eq!(summary.functions["bad"].status, AnalysisStatus::Safe);
    // With only a neutral summary for `bad`, the caller's acquisition is an
    // undischarged obligation rather than one a self-consistent summary
    // explained away.
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert!(has(&summary, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
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

// --- Out parameters --------------------------------------------------------

#[test]
fn an_out_parameter_wrapper_is_inferred_and_validated() {
    // `*out = p` is a store the contract in force sanctions as an out slot;
    // the resource it leaves there is the caller's obligation, not a leak.
    let source = "int fill(void **out) { void *p = q_alloc(); *out = p; return 0; } \
                  void subject(void) { void *p; int rc = fill(&p); q_free(p); }";
    let summary = analyze(source, &[Q_BASE, Q_RICH_MODES]);
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert_eq!(
        summary_in(source, &[Q_BASE, Q_RICH_MODES], "fill"),
        ResourceFunctionContract::new(
            [ResourceParameterMode::Out { nullable: false }],
            ResourceReturnMode::None,
        )
    );

    // The obligation the summary hands the caller is a real one.
    let leaked = analyze(
        "int fill(void **out) { void *p = q_alloc(); *out = p; return 0; } \
         void subject(void) { void *p; int rc = fill(&p); return; }",
        &[Q_BASE, Q_RICH_MODES],
    );
    assert_eq!(leaked.status, AnalysisStatus::Unsafe);
    assert!(has(&leaked, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
}

#[test]
fn an_out_slot_left_unwritten_on_some_path_yields_no_summary() {
    // A path that stores nothing leaves the caller's variable as it was, which
    // no `Out` mode describes: the caller would be told to discharge a
    // resource that was never produced.
    let source = "int fill(void **out, int c) { if (c) { void *p = q_alloc(); *out = p; } return 0; } \
                  void subject(void) { void *p; int rc = fill(&p, 1); q_free(p); }";
    let summary = analyze(source, &[Q_BASE, Q_RICH_MODES]);
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(no_summary(source, &[Q_BASE, Q_RICH_MODES], "fill"));
}

#[test]
fn an_out_slot_nulled_on_a_failure_path_is_nullable() {
    let source = "int fill(void **out, int c) { if (c) { void *p = q_alloc(); *out = p; return 0; } *out = 0; return -1; } \
                  void subject(void) { void *p; int rc = fill(&p, 1); if (p) { q_free(p); } }";
    let summary = analyze(source, &[Q_BASE, Q_RICH_MODES]);
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert_eq!(
        summary_in(source, &[Q_BASE, Q_RICH_MODES], "fill"),
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Out { nullable: true },
                ResourceParameterMode::Borrowed,
            ],
            ResourceReturnMode::None,
        )
    );

    // Without the caller's null check the unit stays unresolved.
    let unchecked = analyze(
        "int fill(void **out, int c) { if (c) { void *p = q_alloc(); *out = p; return 0; } *out = 0; return -1; } \
         void subject(void) { void *p; int rc = fill(&p, 1); q_free(p); }",
        &[Q_BASE, Q_RICH_MODES],
    );
    assert_eq!(unchecked.status, AnalysisStatus::Unknown);
}

#[test]
fn a_possibly_null_out_store_without_a_callee_side_test_yields_no_summary() {
    // The callee stores an acquisition it never resolved. Claiming a non-null
    // slot would be false, and claiming a nullable one would still hide that
    // the caller's null test is the only thing standing between it and a
    // dereference the callee never justified.
    let source = "int fill(void **out) { void *p = q_maybe_alloc(); *out = p; return 0; } \
                  void subject(void) { void *p; int rc = fill(&p); if (p) { q_free(p); } }";
    let summary = analyze(source, &[Q_BASE, Q_NULLABLE, Q_RICH_MODES]);
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(no_summary(
        source,
        &[Q_BASE, Q_NULLABLE, Q_RICH_MODES],
        "fill"
    ));
}

#[test]
fn an_overwritten_out_slot_leaks_the_first_resource() {
    // Only what the slot holds at the exit is the caller's; the resource the
    // second store displaced is a leak in the callee, and the exemption does
    // not cover it: no summary is validated for `fill`.
    //
    // Card W44-LEAK-PIN-AUDIT: the published analysis of `fill` has no out-slot
    // contract, so both stores through `*out` are refused, and each refused
    // store hands its identity on as its value. The refused-escape rule
    // withdraws the accusation that used to be reported there (it rested on a
    // store the projection did not model), leaving `fill` `Unknown`, never
    // `Safe`; the displaced resource still blocks the summary.
    let source = "int fill(void **out) { void *p = q_alloc(); *out = p; void *q = q_alloc(); *out = q; return 0; } \
                  void subject(void) { void *p; int rc = fill(&p); q_free(p); }";
    let summary = analyze(source, &[Q_BASE, Q_RICH_MODES]);
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(!has(&summary, "fill", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
    assert!(no_summary(source, &[Q_BASE, Q_RICH_MODES], "fill"));
}

#[test]
fn a_formal_written_through_an_out_slot_yields_no_summary() {
    // The caller already owns `p`. Reporting it back through the slot as a
    // fresh resource would mint a second custody for one object.
    let source = "int move_it(void *p, void **out) { *out = p; return 0; } \
                  void subject(void) { void *p = q_alloc(); void *q; int rc = move_it(p, &q); q_free(q); }";
    let summary = analyze(source, &[Q_BASE, Q_RICH_MODES]);
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(no_summary(source, &[Q_BASE, Q_RICH_MODES], "move_it"));
}

#[test]
fn a_returned_out_slot_resource_yields_no_summary() {
    // Handing the same resource back twice — once through the slot, once as
    // the result — is the same double custody seen from the other side.
    let source = "void *fill(void **out) { void *p = q_alloc(); *out = p; return p; } \
                  void subject(void) { void *p; void *q = fill(&p); q_free(q); }";
    let summary = analyze(source, &[Q_BASE, Q_RICH_MODES]);
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(no_summary(source, &[Q_BASE, Q_RICH_MODES], "fill"));
}

#[test]
fn a_store_through_a_local_pointer_stays_unmodeled() {
    // Naming `*slot` is not the same as modeling it. Without an out-parameter
    // contract sanctioning the slot the store is as opaque as it ever was.
    let source = "void keep(void **slot) { void *p = q_alloc(); void **local = slot; *local = p; } \
                  void subject(void) { void **slot; keep(slot); }";
    let summary = analyze(source, &[Q_BASE, Q_RICH_MODES]);
    assert_ne!(summary.status, AnalysisStatus::Safe);
    assert!(has(&summary, "keep", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Unsupported { description }
            if description.contains("store through `*local` without an out-parameter contract")
    )));
    assert!(no_summary(source, &[Q_BASE, Q_RICH_MODES], "keep"));
}

// --- Reallocation ----------------------------------------------------------

#[test]
fn a_realloc_wrapper_is_inferred_as_reallocated() {
    // The wrapper passes the reallocation's conditional custody straight
    // through: on success the argument is gone, on failure the caller still
    // holds it, and only the caller's null test tells the two apart.
    let source = "void *grow(void *p, unsigned long n) { void *q = q_realloc(p, n); return q; } \
                  void subject(void) { void *p = q_alloc(); void *q = grow(p, 8); if (q) { q_free(q); } else { q_free(p); } }";
    let summary = analyze(source, &[Q_BASE, Q_REALLOC]);
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert_eq!(
        summary_in(source, &[Q_BASE, Q_REALLOC], "grow"),
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Borrowed,
            ],
            ResourceReturnMode::Reallocated {
                parameter: 0,
                nullable: true,
            },
        )
    );

    // The same reallocation written inline still resolves both outcomes.
    let inline = analyze(
        "void subject(void) { void *p = q_alloc(); void *q = q_realloc(p, 8); if (q) { q_free(q); } else { q_free(p); } }",
        &[Q_BASE, Q_REALLOC],
    );
    assert_eq!(inline.status, AnalysisStatus::Safe);
}

#[test]
fn a_realloc_wrapper_caller_that_also_frees_the_old_pointer_is_unsafe() {
    // On the success path the old pointer was replaced. Freeing it as well is
    // the double release the summary exists to expose.
    let summary = analyze(
        "void *grow(void *p, unsigned long n) { void *q = q_realloc(p, n); return q; } \
         void subject(void) { void *p = q_alloc(); void *q = grow(p, 8); if (q) { q_free(q); q_free(p); } else { q_free(p); } }",
        &[Q_BASE, Q_REALLOC],
    );
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert!(has(&summary, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::DoubleRelease { .. }
    )));
}

#[test]
fn a_realloc_wrapper_that_discards_its_argument_is_not_reallocated() {
    // The classic double-custody trap. This wrapper frees the old pointer on
    // the failure path, so `Reallocated` — which promises the caller keeps it
    // — would be a false summary. The derived summary releases the formal
    // unconditionally instead, and a caller written for realloc semantics is
    // reported rather than certified.
    let source = "void *grow(void *p, unsigned long n) { void *q = q_realloc(p, n); if (q) { return q; } q_free(p); return 0; } \
                  void subject(void) { void *p = q_alloc(); void *q = grow(p, 8); if (q) { q_free(q); } }";
    assert_eq!(
        summary_in(source, &[Q_BASE, Q_REALLOC], "grow"),
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Released { nullable: false },
                ResourceParameterMode::Borrowed,
            ],
            ResourceReturnMode::Owned { nullable: true },
        )
    );
    assert_eq!(
        analyze(source, &[Q_BASE, Q_REALLOC]).status,
        AnalysisStatus::Safe
    );

    let double_freed = analyze(
        "void *grow(void *p, unsigned long n) { void *q = q_realloc(p, n); if (q) { return q; } q_free(p); return 0; } \
         void subject(void) { void *p = q_alloc(); void *q = grow(p, 8); if (q) { q_free(q); } else { q_free(p); } }",
        &[Q_BASE, Q_REALLOC],
    );
    assert_eq!(double_freed.status, AnalysisStatus::Unsafe);
    assert!(has(&double_freed, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::DoubleRelease { .. }
    )));
}

#[test]
fn a_realloc_wrapper_that_resolves_its_own_null_test_is_reallocated() {
    // The callee splits the two outcomes itself: one exit releases the old
    // resource and returns a non-null fresh one, the other keeps the old and
    // returns nothing. That is precisely what `Reallocated` means, and no
    // single parameter mode describes it.
    let source = "void *grow(void *p, unsigned long n) { void *q = q_realloc(p, n); if (q) { return q; } return 0; } \
                  void subject(void) { void *p = q_alloc(); void *q = grow(p, 8); if (q) { q_free(q); } else { q_free(p); } }";
    let summary = analyze(source, &[Q_BASE, Q_REALLOC]);
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert_eq!(
        summary_in(source, &[Q_BASE, Q_REALLOC], "grow"),
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Borrowed,
            ],
            ResourceReturnMode::Reallocated {
                parameter: 0,
                nullable: true,
            },
        )
    );
}

// --- Aggregate custody -----------------------------------------------------

#[test]
fn a_stored_parameter_is_inferred_with_its_owner() {
    let source = "void add(void *owner, void *item) { q_store(owner, item); } \
                  void subject(void) { void *c = q_alloc(); void *i = q_alloc(); add(c, i); q_free(c); }";
    let summary = analyze(source, &[Q_BASE, Q_CONTAINER_STORE]);
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert_eq!(
        summary_in(source, &[Q_BASE, Q_CONTAINER_STORE], "add"),
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Stored {
                    owner_parameter: 0,
                    nullable: false,
                },
            ],
            ResourceReturnMode::None,
        )
    );
}

#[test]
fn releasing_a_stored_item_after_the_call_is_a_custody_conflict() {
    let summary = analyze(
        "void add(void *owner, void *item) { q_store(owner, item); } \
         void subject(void) { void *c = q_alloc(); void *i = q_alloc(); add(c, i); q_free(i); q_free(c); }",
        &[Q_BASE, Q_CONTAINER_STORE],
    );
    assert_eq!(summary.status, AnalysisStatus::Unsafe);
    assert!(has(&summary, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::CustodyConflict { .. }
    )));
}

#[test]
fn a_removal_wrapper_restores_direct_custody_for_its_caller() {
    let source = "void take(void *owner, void *item) { q_remove(owner, item); } \
                  void subject(void) { void *c = q_alloc(); void *i = q_alloc(); q_store(c, i); take(c, i); q_free(c); q_free(i); }";
    let modules = &[Q_BASE, Q_CONTAINER_STORE, Q_CONTAINER_REMOVE];
    assert_eq!(analyze(source, modules).status, AnalysisStatus::Safe);
    assert_eq!(
        summary_in(source, modules, "take"),
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Removed {
                    owner_parameter: 0,
                    nullable: false
                }
            ],
            ResourceReturnMode::None
        )
    );
}

// --- Nullable results ------------------------------------------------------

#[test]
fn a_wrapper_returning_an_unresolved_acquisition_is_nullable() {
    // A body may return a possibly-null place only under a result mode that
    // obliges its callers to null-check; the mode is then re-checked against a
    // run that grants exactly that permission and no more.
    let source = "void *make(void) { void *p = q_maybe_alloc(); return p; } \
                  void subject(void) { void *p = make(); if (p) { q_free(p); } }";
    let summary = analyze(source, &[Q_BASE, Q_NULLABLE]);
    assert_eq!(summary.status, AnalysisStatus::Safe);
    assert_eq!(
        summary_in(source, &[Q_BASE, Q_NULLABLE], "make"),
        ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: true })
    );

    let unchecked = analyze(
        "void *make(void) { void *p = q_maybe_alloc(); return p; } \
         void subject(void) { void *p = make(); q_free(p); }",
        &[Q_BASE, Q_NULLABLE],
    );
    assert_eq!(unchecked.status, AnalysisStatus::Unknown);
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

#[test]
fn reassigned_out_formals_do_not_describe_the_callers_slot() {
    for assignment in ["out = 0;", "out = q_alloc();", "q_make(&out);"] {
        let source = format!(
            "int fill(void **out) {{ {assignment} void *p = q_alloc(); *out = p; return 0; }} void subject(void) {{ void *p; int rc = fill(&p); q_free(p); }}"
        );
        assert!(
            no_summary(&source, &[Q_BASE, Q_RICH_MODES], "fill"),
            "{assignment}"
        );
        assert_ne!(
            analyze(&source, &[Q_BASE, Q_RICH_MODES]).status,
            AnalysisStatus::Safe
        );
    }
}

#[test]
fn extended_summary_certificates_are_rederived_and_reject_tampering() {
    use crate::legacy_c::{
        C11TranslationUnit, UnrestrictedLegacyResult, analyze_unrestricted_c11,
        check_legacy_safety_certificate,
    };
    if !crate::test_support::clang_available() {
        return;
    }
    for source in [
        "void *q_alloc(void); void q_free(void *); int fill(void **out) { void *p = q_alloc(); *out = p; return 0; } void subject(void) { void *p; int rc = fill(&p); q_free(p); }",
        "void *q_alloc(void); void q_free(void *); void *q_realloc(void *, __SIZE_TYPE__); void *grow(void *p) { void *q = q_realloc(p, 8); return q; } void subject(void) { void *p = q_alloc(); void *q = grow(p); if (q) { q_free(q); } else { q_free(p); } }",
    ] {
        let unit = C11TranslationUnit {
            source,
            ..C11TranslationUnit::default()
        };
        let result =
            analyze_unrestricted_c11(&unit, "subject", &fixture_contracts(&[Q_BASE, Q_REALLOC]))
                .unwrap();
        let UnrestrictedLegacyResult::Safe(mut certificate) = result else {
            panic!("{result:?}")
        };
        check_legacy_safety_certificate(&certificate).unwrap();
        let record = certificate
            .summaries
            .iter_mut()
            .find(|record| record.function != "subject")
            .unwrap();
        record.contract = ResourceFunctionContract::new([], ResourceReturnMode::None);
        assert!(check_legacy_safety_certificate(&certificate).is_err());
    }
}

#[test]
fn removal_summary_composes_through_reordered_direct_formals() {
    let source = "void take(void *owner, void *item) { q_remove(owner, item); } void reverse(void *item, void *owner) { take(owner, item); }";
    assert_eq!(
        summary_in(source, &[Q_BASE, Q_CONTAINER_REMOVE], "reverse"),
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Removed {
                    owner_parameter: 1,
                    nullable: false
                },
                ResourceParameterMode::Owned { nullable: false }
            ],
            ResourceReturnMode::None
        )
    );
}

#[test]
fn a_store_then_remove_keeps_its_live_entry_contract() {
    let source =
        "void roundtrip(void *owner, void *item) { q_store(owner, item); q_remove(owner, item); }";
    assert_eq!(
        summary_in(
            source,
            &[Q_BASE, Q_CONTAINER_STORE, Q_CONTAINER_REMOVE],
            "roundtrip"
        ),
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Owned { nullable: false }
            ],
            ResourceReturnMode::None
        )
    );
}

#[test]
fn removed_formal_identity_and_all_exit_dispositions_are_rechecked() {
    for body in [
        "item = other; q_remove(owner, item);",
        "owner = other; q_remove(owner, item);",
        "item = 0; q_remove(owner, item);",
        "owner = 0; q_remove(owner, item);",
        "item = owner; q_remove(owner, item);",
        "if (flag) { q_remove(owner, item); }",
        "q_remove(owner, item); q_remove(owner, item);",
        "q_remove(owner, item); q_store(owner, item);",
        "q_remove(owner, item); q_free(item);",
        "q_remove(owner, item); q_free(owner);",
        "q_remove(owner, item); opaque(item);",
        "q_remove(owner, item); return item;",
        "q_remove(other, item); q_remove(owner, item);",
    ] {
        let source =
            format!("void take(void *owner, void *item, void *other, int flag) {{ {body} }}");
        assert!(
            no_summary(
                &source,
                &[Q_BASE, Q_CONTAINER_STORE, Q_CONTAINER_REMOVE],
                "take"
            ),
            "accepted {body}"
        );
    }
}

#[test]
fn general_alias_removal_discovery_remains_unsupported() {
    for body in [
        "void *alias = item; q_remove(owner, alias);",
        "void *alias = owner; q_remove(alias, item);",
        "q_remove(owner, item->child);",
    ] {
        let source = format!("void take(void *owner, void *item) {{ {body} }}");
        assert!(
            no_summary(&source, &[Q_BASE, Q_CONTAINER_REMOVE], "take"),
            "accepted {body}"
        );
    }
    // Pin sharpened by card C03-NFR. A self-recursive body that removes nothing
    // is now published by the neutral-frame route — but only as the neutral
    // contract. No removal is discovered from the recursion, which is what this
    // case exists to refuse.
    let recursive = "void take(void *owner, void *item) { take(owner, item); }";
    assert_eq!(
        summary_in(recursive, &[Q_BASE, Q_CONTAINER_REMOVE], "take"),
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Borrowed,
                ResourceParameterMode::Borrowed
            ],
            ResourceReturnMode::None
        )
    );
}

#[test]
fn nullable_removal_validates_null_and_nonnull_exits_and_forwarding() {
    let nullable = Q_CONTAINER_REMOVE.1.replace(
        "resource_removed(item, owner, nonnull)",
        "resource_removed(item, owner, nullable)",
    );
    for (body, remove_contract) in [
        ("q_remove(owner, item);", nullable.as_str()),
        ("if (item) { q_remove(owner, item); }", Q_CONTAINER_REMOVE.1),
        (
            "if (!item) { return; } q_remove(owner, item);",
            Q_CONTAINER_REMOVE.1,
        ),
    ] {
        let source = format!(
            "void take(void *owner, void *item) {{ {body} }} void reverse(void *item, void *owner) {{ take(owner, item); }}"
        );
        assert_eq!(
            summary_in(
                &source,
                &[Q_BASE, ("remove.qcore", remove_contract)],
                "reverse"
            ),
            ResourceFunctionContract::new(
                [
                    ResourceParameterMode::Removed {
                        owner_parameter: 1,
                        nullable: true
                    },
                    ResourceParameterMode::Owned { nullable: false }
                ],
                ResourceReturnMode::None,
            ),
            "{body}"
        );
    }
}

#[test]
fn nullable_removal_keeps_siblings_and_independent_entry_modes() {
    let source = "void take(void *owner, void *item, void *sibling) { if (item) { q_remove(owner, item); } q_remove(owner, sibling); }";
    assert_eq!(
        summary_in(source, &[Q_BASE, Q_CONTAINER_REMOVE], "take"),
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Removed {
                    owner_parameter: 0,
                    nullable: true
                },
                ResourceParameterMode::Removed {
                    owner_parameter: 0,
                    nullable: false
                }
            ],
            ResourceReturnMode::None,
        )
    );
    let source = "void take(void *owner, void *item, void *sibling) { q_remove(owner, item); if (sibling) { q_remove(owner, sibling); } }";
    assert_eq!(
        summary_in(source, &[Q_BASE, Q_CONTAINER_REMOVE], "take").parameters[1],
        ResourceParameterMode::Removed {
            owner_parameter: 0,
            nullable: false
        }
    );
    assert_eq!(
        summary_in(source, &[Q_BASE, Q_CONTAINER_REMOVE], "take").parameters[2],
        ResourceParameterMode::Removed {
            owner_parameter: 0,
            nullable: true
        }
    );
}

#[test]
fn partial_and_repeated_nullable_removal_do_not_publish() {
    let nullable = Q_CONTAINER_REMOVE.1.replace(
        "resource_removed(item, owner, nonnull)",
        "resource_removed(item, owner, nullable)",
    );
    for body in [
        "if (flag) { q_remove(owner, item); }",
        "if (flag) { q_remove(owner, item); } q_remove(owner, item);",
        "if (item) { item = other; q_remove(owner, item); }",
        "if (item) { owner = other; q_remove(owner, item); }",
        "if (item) { q_remove(owner, item); q_free(item); }",
    ] {
        let source =
            format!("void take(void *owner, void *item, void *other, int flag) {{ {body} }}");
        for remove in [Q_CONTAINER_REMOVE.1, nullable.as_str()] {
            assert!(
                no_summary(&source, &[Q_BASE, ("remove.qcore", remove)], "take"),
                "accepted {body}"
            );
        }
    }
    // Clearing a borrowed argument before a nullable no-op is valid ordinary
    // behavior, but must not claim to detach the original argument's resource.
    let cleared = summary_in(
        "void take(void *owner, void *item) { item = 0; q_remove(owner, item); }",
        &[Q_BASE, ("remove.qcore", &nullable)],
        "take",
    );
    assert_eq!(cleared.parameters[1], ResourceParameterMode::Borrowed);
}

#[test]
fn invalid_null_path_does_not_gain_a_nullable_precondition() {
    for body in [
        "if (!item) { q_free(owner); return; } q_remove(owner, item);",
        "if (!item) { opaque(owner); return; } q_remove(owner, item);",
    ] {
        let source = format!("void take(void *owner, void *item) {{ {body} }}");
        let summary = summary_in(&source, &[Q_BASE, Q_CONTAINER_REMOVE], "take");
        assert_eq!(
            summary.parameters[1],
            ResourceParameterMode::Removed {
                owner_parameter: 0,
                nullable: false
            }
        );
    }
}

#[test]
fn joined_alternative_containing_owners_cannot_authorize_detachment() {
    let source = "void subject(int flag) { void *a = q_alloc(); void *b = q_alloc(); void *p = q_alloc(); if (flag) { q_store(a, p); } else { q_store(b, p); } q_remove(a, p); q_free(p); q_free(a); q_free(b); }";
    let result = analyze(source, &[Q_BASE, Q_CONTAINER_STORE, Q_CONTAINER_REMOVE]);
    assert_ne!(result.status, AnalysisStatus::Safe);
    assert!(has(&result, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::CustodyConflict { .. }
    )));
}

#[test]
fn joined_child_alias_cannot_detach_all_possible_targets() {
    let source = "void subject(int flag) { void *o = q_alloc(); void *p = q_alloc(); void *q = q_alloc(); void *x; q_store(o, p); q_store(o, q); if (flag) { x = p; } else { x = q; } q_remove(o, x); q_free(p); q_free(q); q_free(o); }";
    let result = analyze(source, &[Q_BASE, Q_CONTAINER_STORE, Q_CONTAINER_REMOVE]);
    assert_ne!(result.status, AnalysisStatus::Safe);
    assert!(has(&result, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::UnknownAlias { .. }
    )));
}

#[test]
fn one_selected_resource_action_cannot_discharge_both_alias_targets() {
    for operation in [
        "q_free(x); q_free(o);",
        "q_store(o, x); q_free(o);",
        "void *r = q_realloc(x, 8); if (r) { q_free(r); } else { q_free(x); } q_free(o);",
    ] {
        let source = format!(
            "void subject(int flag) {{ void *o = q_alloc(); void *p = q_alloc(); void *q = q_alloc(); void *x; if (flag) {{ x = p; }} else {{ x = q; }} {operation} }}"
        );
        let result = analyze(&source, &[Q_BASE, Q_CONTAINER_STORE, Q_REALLOC]);
        assert_ne!(result.status, AnalysisStatus::Safe, "{operation}");
        assert!(
            has(&result, "subject", |kind| matches!(
                kind,
                LegacyDiagnosticKind::UnknownAlias { .. }
            )),
            "{operation}"
        );
        assert_eq!(result.status, AnalysisStatus::Unknown, "{operation}");
    }
}

#[test]
fn uncertain_selected_release_does_not_hide_an_independent_leak() {
    let source = "void subject(int flag) { void *leaked = q_alloc(); void *p = q_alloc(); void *q = q_alloc(); void *x; if (flag) { x = p; } else { x = q; } q_free(x); }";
    let result = base(source);
    assert_eq!(result.status, AnalysisStatus::Unsafe);
    assert!(has(&result, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
    assert!(has(&result, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::UnknownAlias { .. }
    )));
}

#[test]
fn conditional_containment_cannot_erase_direct_custody_during_cascade() {
    let source = "void subject(int flag) { void *o = q_alloc(); void *p = q_alloc(); if (flag) { q_store(o, p); } q_free(o); }";
    let result = analyze(source, &[Q_BASE, Q_CONTAINER_STORE]);
    assert_eq!(result.status, AnalysisStatus::Unknown);
    assert!(has(&result, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::UnknownAlias { .. }
    )));
}

#[test]
fn consumed_resources_require_live_direct_custody() {
    for body in ["q_free(p); q_transfer(p);", "q_transfer(p); q_transfer(p);"] {
        let result = base(&format!(
            "void subject(void) {{ void *p = q_alloc(); {body} }}"
        ));
        assert_eq!(result.status, AnalysisStatus::Unsafe, "{body}");
        assert!(has(&result, "subject", |kind| matches!(
            kind,
            LegacyDiagnosticKind::CustodyConflict { .. }
        )));
    }
}

#[test]
fn reallocation_requires_live_custody_on_every_incoming_path() {
    let nonnull = Q_REALLOC.1.replace(
        "resource_return_reallocated(p, nullable)",
        "resource_return_reallocated(p, nonnull)",
    );
    let source = "void subject(int flag) { void *p = q_alloc(); if (flag) { q_free(p); } void *r = q_realloc(p, 8); q_free(r); }";
    let result = analyze(source, &[Q_BASE, ("realloc.qcore", &nonnull)]);
    assert_eq!(result.status, AnalysisStatus::Unknown);
    assert!(has(&result, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::UnknownAlias { .. }
    )));
}

#[test]
fn removal_callers_must_supply_the_exact_contained_child_and_keep_its_obligation() {
    let prefix = "void take(void *owner, void *item) { q_remove(owner, item); }";
    for body in [
        "void *o = q_alloc(); void *p = q_alloc(); take(o, p); q_free(o); q_free(p);",
        "void *o = q_alloc(); void *p = q_alloc(); void *other = q_alloc(); q_store(o, p); take(other, p); q_free(o); q_free(other);",
        "void *o = q_alloc(); void *p = q_alloc(); q_store(o, p); take(o, p); q_free(o);",
        "void *o = q_alloc(); void *p = q_alloc(); q_store(o, p); q_free(o); take(o, p);",
        "void *o = q_alloc(); void *p = q_alloc(); q_store(o, p); take(o, p); take(o, p); q_free(o); q_free(p);",
        "void *o = q_alloc(); take(o, o); q_free(o);",
        "void *o = q_alloc(); take(o, 0); q_free(o);",
    ] {
        let source = format!("{prefix} void subject(void) {{ {body} }}");
        assert_ne!(
            analyze(&source, &[Q_BASE, Q_CONTAINER_STORE, Q_CONTAINER_REMOVE]).status,
            AnalysisStatus::Safe,
            "accepted {body}"
        );
    }
    let source = format!(
        "{prefix} void subject(void) {{ void *o = q_alloc(); void *p = q_alloc(); q_store(o, p); take(o, p); q_free(o); }}"
    );
    let result = analyze(&source, &[Q_BASE, Q_CONTAINER_STORE, Q_CONTAINER_REMOVE]);
    assert!(has(&result, "subject", |kind| matches!(
        kind,
        LegacyDiagnosticKind::Leak { .. }
    )));
}

#[test]
fn removed_summary_keeps_declared_precedence_and_records_conflicts() {
    let declared = Q_CONTAINER_REMOVE.1.replace("q_remove(", "take(").replace(
        "resource_removed(item, owner, nonnull)",
        "resource_owned(item, nonnull)",
    );
    let source = "void take(void *owner, void *item) { q_remove(owner, item); } void subject(void) { return; }";
    let result = analyze(
        source,
        &[Q_BASE, Q_CONTAINER_REMOVE, ("take.qcore", &declared)],
    );
    assert!(has(&result, "take", |kind| matches!(
        kind,
        LegacyDiagnosticKind::SummaryConflict { .. }
    )));
    let tokens = tokenize(source);
    let names = legacy_c_source_functions(source);
    let contracts = fixture_contracts(&[Q_BASE, Q_CONTAINER_REMOVE, ("take.qcore", &declared)]);
    let derived = derive_custody_summaries(&tokens, &names, &contracts);
    assert_eq!(
        derived.environment().contract("take"),
        contracts.contract("take")
    );
    assert!(derived.records().iter().all(|r| r.function != "take"));
}

#[test]
fn removed_certificate_rederives_owner_and_child_effects() {
    use crate::legacy_c::{
        C11TranslationUnit, UnrestrictedLegacyResult, analyze_unrestricted_c11,
        check_legacy_safety_certificate,
    };
    if !crate::test_support::clang_available() {
        return;
    }
    let source = "void *q_alloc(void); void q_free(void *); void q_store(void *, void *); void q_remove(void *, void *); void take(void *owner, void *item) { q_remove(owner, item); } void subject(void) { void *o = q_alloc(); void *p = q_alloc(); q_store(o, p); take(o, p); q_free(o); q_free(p); }";
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
        &fixture_contracts(&[Q_BASE, Q_CONTAINER_STORE, Q_CONTAINER_REMOVE]),
    )
    .unwrap();
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("{result:?}")
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    let mut changed_source = certificate.clone();
    changed_source.preprocessed_source = changed_source
        .preprocessed_source
        .replace("q_remove(", "unknown_remove(");
    assert_ne!(
        changed_source.preprocessed_source,
        certificate.preprocessed_source
    );
    assert!(check_legacy_safety_certificate(&changed_source).is_err());
    for mode in [
        ResourceParameterMode::Owned { nullable: false },
        ResourceParameterMode::Removed {
            owner_parameter: 1,
            nullable: false,
        },
        ResourceParameterMode::Removed {
            owner_parameter: 0,
            nullable: true,
        },
    ] {
        let mut changed = certificate.clone();
        changed
            .summaries
            .iter_mut()
            .find(|record| record.function == "take")
            .unwrap()
            .contract
            .parameters[1] = mode;
        assert!(check_legacy_safety_certificate(&changed).is_err());
    }
}

#[test]
fn removal_preserves_other_children_and_rejects_aliased_removal_arguments() {
    let modules = &[Q_BASE, Q_CONTAINER_STORE, Q_CONTAINER_REMOVE];
    let source = "void take(void *owner, void *item) { q_remove(owner, item); } void subject(void) { void *o = q_alloc(); void *p = q_alloc(); void *q = q_alloc(); q_store(o, p); q_store(o, q); take(o, p); q_free(o); q_free(p); }";
    assert_eq!(analyze(source, modules).status, AnalysisStatus::Safe);
    let prefix = "void take2(void *owner, void *first, void *second) { q_remove(owner, first); q_remove(owner, second); }";
    let valid = format!(
        "{prefix} void subject(void) {{ void *o = q_alloc(); void *p = q_alloc(); void *q = q_alloc(); q_store(o, p); q_store(o, q); take2(o, p, q); q_free(o); q_free(p); q_free(q); }}"
    );
    assert_eq!(analyze(&valid, modules).status, AnalysisStatus::Safe);
    assert_ne!(
        analyze(&valid.replace("take2(o, p, q)", "take2(o, p, p)"), modules).status,
        AnalysisStatus::Safe
    );
}

#[test]
fn direct_out_forwarding_initializes_the_callers_original_slot() {
    let source = "void fill(void **out) { q_make(out); } void subject(void) { void *p; fill(&p); q_free(p); }";
    let modules = &[Q_BASE, Q_RICH_MODES];
    assert_eq!(analyze(source, modules).status, AnalysisStatus::Safe);
    assert_eq!(
        summary_in(source, modules, "fill"),
        ResourceFunctionContract::new(
            [ResourceParameterMode::Out { nullable: false }],
            ResourceReturnMode::None
        )
    );
}

#[test]
fn forwarded_out_slots_compose_through_acyclic_wrappers_and_parentheses() {
    let source = "void fill(void **out) { q_make((out)); } void relay(void **out) { fill(out); } void subject(void) { void *p; relay(&p); q_free(p); }";
    let modules = &[Q_BASE, Q_RICH_MODES];
    assert_eq!(analyze(source, modules).status, AnalysisStatus::Safe);
    assert_eq!(
        summary_in(source, modules, "relay"),
        ResourceFunctionContract::new(
            [ResourceParameterMode::Out { nullable: false }],
            ResourceReturnMode::None
        )
    );
}

#[test]
fn forwarding_does_not_accept_reassigned_escaping_or_unwritten_slots() {
    for body in [
        "{ void **out; q_make(out); }",
        "{ void *local, **out; q_make(out); }",
        "{ void **out = other; q_make(out); }",
        "{ void *local = q_alloc(), **out; q_free(local); q_make(out); }",
        "out = other; q_make(out);",
        "out++; q_make(out);",
        "--out; q_make(out);",
        "q_observe(out = other); q_make(out);",
        "q_observe(&out); q_make(out);",
        "q_make(out); q_observe(out);",
        "q_make(out); q_observe((void *)out);",
        "q_make(out); q_observe(out + 0);",
        "q_make(out), q_observe(out);",
        "flag && (q_make(out), 1);",
        "out = 0; q_make(out);",
        "q_make(&out); q_make(out);",
        "q_make(out); out = other;",
        "void **alias = out; alias = other; q_make(alias);",
        "void **alias = out; q_make(out); q_observe(alias);",
        "if (flag) { q_make(out); }",
        "q_make(out); q_make(out);",
        "q_make(out); *out = 0;",
        "q_make(out + 1);",
        "q_make(0);",
        "q_make(out); opaque(out);",
    ] {
        let source = format!("void fill(void **out, void **other, int flag) {{ {body} }}");
        assert!(
            no_summary(&source, &[Q_BASE, Q_RICH_MODES], "fill"),
            "accepted {body}"
        );
    }
    // A bare local pointer never receives an inferred caller-slot authority.
    let source = "void subject(void) { void *p; q_make(p); q_free(p); }";
    let result = analyze(source, &[Q_BASE, Q_RICH_MODES]);
    assert_ne!(result.status, AnalysisStatus::Safe);
    assert!(has(
        &result,
        "subject",
        |kind| matches!(kind, LegacyDiagnosticKind::Unsupported { description } if description.contains("without an out-parameter contract"))
    ));
}

#[test]
fn forwarded_outputs_keep_nullability_and_dont_invent_unresolved_handback() {
    let source = "void fill(void **out, int flag) { if (flag) { q_make(out); return; } *out = 0; } void subject(void) { void *p; fill(&p, 1); if (p) { q_free(p); } }";
    let modules = &[Q_BASE, Q_RICH_MODES];
    assert_eq!(analyze(source, modules).status, AnalysisStatus::Safe);
    assert_eq!(
        summary_in(source, modules, "fill").parameters[0],
        ResourceParameterMode::Out { nullable: true }
    );
    assert_ne!(
        analyze(
            &source.replace("if (p) { q_free(p); }", "q_free(p);"),
            modules
        )
        .status,
        AnalysisStatus::Safe
    );
    let nullable = Q_RICH_MODES.1.replace(
        "resource_out(destination, nonnull)",
        "resource_out(destination, nullable)",
    );
    assert!(no_summary(
        "void fill(void **out) { q_make(out); }",
        &[Q_BASE, ("nullable-out.qcore", &nullable)],
        "fill"
    ));
}

#[test]
fn forwarded_output_callers_keep_leak_and_slot_alias_obligations() {
    let prefix = "void fill(void **a, void **b) { q_make(a); q_make(b); }";
    let source = format!(
        "{prefix} void subject(void) {{ void *p; void *q; fill(&p, &q); q_free(p); q_free(q); }}"
    );
    let modules = &[Q_BASE, Q_RICH_MODES];
    assert_eq!(analyze(&source, modules).status, AnalysisStatus::Safe);
    for body in [
        "void *p; void *q; fill(&p, &q); q_free(p);",
        "void *p; fill(&p, &p); q_free(p);",
        "void *p = q_alloc(); void *q; fill(&p, &q); q_free(p); q_free(q);",
    ] {
        let result = analyze(
            &format!("{prefix} void subject(void) {{ {body} }}"),
            modules,
        );
        assert!(
            has(&result, "subject", |kind| matches!(
                kind,
                LegacyDiagnosticKind::Leak { .. }
            )),
            "{body}"
        );
    }
}

#[test]
fn out_forwarding_respects_declared_contracts_and_certificate_rederivation() {
    use crate::legacy_c::{
        C11TranslationUnit, UnrestrictedLegacyResult, analyze_unrestricted_c11,
        check_legacy_safety_certificate,
    };
    let source = "void q_make(void **); void q_free(void *); void fill(void **out) { q_make(out); } void subject(void) { void *p; fill(&p); q_free(p); }";
    let declaration = "module contract; extern fn fill(out: ptr<mut, ptr<mut, u8>>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(out); ensures normal resource_return_none(); };";
    let result = analyze(source, &[Q_BASE, Q_RICH_MODES, ("fill.qcore", declaration)]);
    assert!(has(&result, "fill", |kind| matches!(
        kind,
        LegacyDiagnosticKind::SummaryConflict { .. }
    )));
    if !crate::test_support::clang_available() {
        return;
    }
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
        &fixture_contracts(&[Q_BASE, Q_RICH_MODES]),
    )
    .unwrap();
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("{result:?}")
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    let shadowed = source.replace("q_make(out);", "{ void **out; q_make(out); }");
    let shadowed_result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &shadowed,
            ..Default::default()
        },
        "subject",
        &fixture_contracts(&[Q_BASE, Q_RICH_MODES]),
    )
    .unwrap();
    assert!(!matches!(
        shadowed_result,
        UnrestrictedLegacyResult::Safe(_)
    ));
    for mode in [
        ResourceParameterMode::Borrowed,
        ResourceParameterMode::Out { nullable: true },
    ] {
        let mut changed = certificate.clone();
        changed
            .summaries
            .iter_mut()
            .find(|record| record.function == "fill")
            .unwrap()
            .contract
            .parameters[0] = mode;
        assert!(check_legacy_safety_certificate(&changed).is_err());
    }
    let mut changed = certificate.clone();
    changed.preprocessed_source = changed
        .preprocessed_source
        .replace("q_make(out)", "q_make(&out)");
    assert_ne!(changed.preprocessed_source, certificate.preprocessed_source);
    assert!(check_legacy_safety_certificate(&changed).is_err());
}

#[test]
fn formal_identity_guards_keep_scalar_updates_but_reject_resource_mutation() {
    let source = "void fill(void **out, int count) { count++; q_make(out); } void subject(void) { void *p; fill(&p, 0); q_free(p); }";
    assert_eq!(
        analyze(source, &[Q_BASE, Q_RICH_MODES]).status,
        AnalysisStatus::Safe
    );
    for body in ["p++; q_free(p);", "{ void *p; q_free(p); }"] {
        assert!(no_summary(
            &format!("void drop(void *p) {{ {body} }}"),
            &[Q_BASE],
            "drop"
        ));
    }
}

#[test]
fn resource_aliases_do_not_hide_formal_pointer_mutation() {
    let local =
        "void subject(void) { void *p = q_alloc(); void *alias = p; alias++; q_free(alias); }";
    assert_ne!(base(local).status, AnalysisStatus::Safe);
    for body in [
        "p++; void *alias = p; q_free(alias);",
        "p++; void *a = p; void *b = a; q_free(b);",
        "void *alias = p; p++; q_free(p);",
        "void *alias = p; alias++; q_free(alias);",
    ] {
        assert!(
            no_summary(
                &format!("void drop(void *p) {{ {body} }}"),
                &[Q_BASE],
                "drop"
            ),
            "accepted {body}"
        );
    }
}

#[test]
fn immutable_out_aliases_and_chains_keep_original_caller_slots() {
    let modules = &[Q_BASE, Q_RICH_MODES];
    for body in [
        "void **alias=out; q_make(alias);",
        "void **first=out; void **last=first; q_make((last));",
        "void **alias=out; *alias=q_alloc();",
    ] {
        let source = format!(
            "void fill(void **out) {{ {body} }} void subject(void) {{ void *p; fill(&p); q_free(p); }}"
        );
        assert_eq!(
            analyze(&source, modules).status,
            AnalysisStatus::Safe,
            "{body}"
        );
        assert_eq!(
            summary_in(&source, modules, "fill").parameters,
            [ResourceParameterMode::Out { nullable: false }]
        );
    }
    let source = "void fill(void **a,void **b) {void **x=a;void **y=b;q_make(x);q_make(y);} void subject(void){void *p;void *q;fill(&p,&q);q_free(p);q_free(q);}";
    assert_eq!(analyze(source, modules).status, AnalysisStatus::Safe);
    assert_eq!(
        summary_in(source, modules, "fill").parameters,
        vec![ResourceParameterMode::Out { nullable: false }; 2]
    );
}

#[test]
fn immutable_out_aliases_compose_and_preserve_nullable_exit_checks() {
    let modules = &[Q_BASE, Q_RICH_MODES];
    let chained = "void fill(void **out){void **alias=out;q_make(alias);} void relay(void **out){void **alias=out;fill(alias);} void subject(void){void *p;relay(&p);q_free(p);}";
    assert_eq!(analyze(chained, modules).status, AnalysisStatus::Safe);
    assert_eq!(
        summary_in(chained, modules, "relay").parameters,
        [ResourceParameterMode::Out { nullable: false }]
    );
    let source = "void fill(void **out,int flag) {void **alias=out;if(flag){q_make(alias);return;}*alias=0;} void subject(void){void *p;fill(&p,1);if(p){q_free(p);}}";
    assert_eq!(analyze(source, modules).status, AnalysisStatus::Safe);
    assert_eq!(
        summary_in(source, modules, "fill").parameters[0],
        ResourceParameterMode::Out { nullable: true }
    );
    assert_ne!(
        analyze(&source.replace("if(p){q_free(p);}", "q_free(p);"), modules).status,
        AnalysisStatus::Safe
    );
    assert!(no_summary(
        &source.replace("*alias=0;", ""),
        modules,
        "fill"
    ));
    assert!(no_summary(
        "void fill(void **out,int flag){void **alias=out;if(flag){q_make(alias);return;}*alias=0;} void relay(void **out){void **alias=out;fill(alias,1);}",
        modules,
        "relay"
    ));
}

#[test]
fn out_alias_normalization_rejects_mutation_shadow_escape_and_type_changes() {
    let modules = &[Q_BASE, Q_RICH_MODES];
    for body in [
        "void **alias=out;alias=other;q_make(alias);",
        "void **alias=out;alias++;q_make(alias);",
        "void **alias=out;out=other;q_make(alias);",
        "void **alias=out;out++;q_make(alias);",
        "void **alias=out;q_make(alias);q_observe(alias);",
        "void **alias=out;q_observe(&alias);q_make(alias);",
        "void **alias=out;q_make(&alias);q_make(alias);",
        "void **alias=out;{void **alias=other;q_make(alias);}",
        "void **alias=out;{void *first,**alias;q_make(alias);}",
        "void **alias=out;{void **out=other;q_make(alias);}",
        "void **alias=out;void **alias=other;q_make(alias);",
        "int **alias=out;q_make(alias);",
        "void **volatile alias=out;q_make(alias);",
        "static void **alias=out;q_make(alias);",
        "void **alias=(void **)out;q_make(alias);",
        "if(flag){void **alias=out;q_make(alias);}",
        "void **alias;alias=out;q_make(alias);",
        "void **alias=out;void *p=*alias;q_free(p);q_make(alias);",
        "void **alias=out;if(alias){q_make(alias);}",
        "void **alias=out;flag && (q_make(alias),1);",
        "void **alias=out;q_make(alias),q_observe(out);",
    ] {
        let source = format!("void fill(void **out,void **other,int flag){{{body}}}");
        assert!(no_summary(&source, modules, "fill"), "accepted {body}");
    }
}

#[test]
fn aliased_out_callers_still_discharge_distinct_fresh_outputs() {
    let modules = &[Q_BASE, Q_RICH_MODES];
    let prefix = "void fill(void **a,void **b){void **x=a;void **y=b;q_make(x);q_make(y);}";
    for body in [
        "void *p;void *q;fill(&p,&q);q_free(p);",
        "void *p;fill(&p,&p);q_free(p);",
        "void *p=q_alloc();void *q;fill(&p,&q);q_free(p);q_free(q);",
    ] {
        let result = analyze(&format!("{prefix}void subject(void){{{body}}}"), modules);
        assert!(
            has(&result, "subject", |kind| matches!(
                kind,
                LegacyDiagnosticKind::Leak { .. }
            )),
            "{body}"
        );
    }
    assert!(no_summary(
        "void fill(void **out){void **x=out;void **y=out;q_make(x);q_make(y);}",
        modules,
        "fill"
    ));
}

#[test]
fn aliased_out_inference_preserves_declared_precedence() {
    let source = "void fill(void **out){void **alias=out;q_make(alias);}void subject(void){void *p;fill(&p);q_free(p);}";
    let declared = "module contract; extern fn fill(out: ptr<mut, ptr<mut, u8>>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(out); ensures normal resource_return_none(); };";
    let result = analyze(source, &[Q_BASE, Q_RICH_MODES, ("fill.qcore", declared)]);
    assert!(has(&result, "fill", |kind| matches!(
        kind,
        LegacyDiagnosticKind::SummaryConflict { .. }
    )));
    assert_ne!(result.status, AnalysisStatus::Safe);
}

#[test]
fn real_c_alias_certificate_rederives_identity_and_rejects_changed_source_or_summary() {
    use crate::legacy_c::{
        C11TranslationUnit, UnrestrictedLegacyResult, analyze_unrestricted_c11,
        check_legacy_safety_certificate,
    };
    if !crate::test_support::clang_available() {
        return;
    }
    let source = include_str!("../fixtures/out_alias_wrappers.c");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
        &fixture_contracts(&[Q_BASE, Q_RICH_MODES]),
    )
    .unwrap();
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("{result:?}")
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    let mut changed = certificate.clone();
    changed.preprocessed_source = changed
        .preprocessed_source
        .replace("q_make(alias)", "alias++;q_make(alias)");
    assert_ne!(changed.preprocessed_source, certificate.preprocessed_source);
    assert!(check_legacy_safety_certificate(&changed).is_err());
    let mut changed = certificate.clone();
    changed
        .summaries
        .iter_mut()
        .find(|s| s.function == "fill")
        .unwrap()
        .contract
        .parameters[0] = ResourceParameterMode::Borrowed;
    assert!(check_legacy_safety_certificate(&changed).is_err());
}

// --- C01: owner identity and transitive re-derivation ----------------------

#[test]
fn a_stored_owner_must_be_the_same_caller_held_formal_at_every_exit() {
    // `Stored` names *which* caller-held formal took the item. Reading it off
    // the exit containment means the position follows the body, not the
    // argument order of the modeled call; and an owner that differs between
    // exits — or is the child itself — describes a transfer no single mode
    // expresses, so nothing is published.
    let modules = &[Q_BASE, Q_CONTAINER_STORE];
    assert_eq!(
        summary_in(
            "void add(void *item, void *owner) { q_store(owner, item); }",
            modules,
            "add",
        ),
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Stored {
                    owner_parameter: 1,
                    nullable: false,
                },
                ResourceParameterMode::Owned { nullable: false },
            ],
            ResourceReturnMode::None,
        )
    );
    for body in [
        "if (flag) { q_store(first, item); } else { q_store(second, item); }",
        "if (flag) { q_store(first, item); }",
        "q_store(first, item); q_store(second, item);",
        "q_store(item, item);",
        "q_store(first, second); q_store(second, item);",
    ] {
        let source =
            format!("void add(void *first, void *second, void *item, int flag) {{ {body} }}");
        assert!(no_summary(&source, modules, "add"), "accepted {body}");
    }
}

#[test]
fn a_changed_transitive_callee_redirects_every_derived_summary() {
    // A calls B calls C, and only C changes. Each derivation is bottom-up over
    // a fresh analysis, so B's published summary and A's verdict both follow C
    // rather than reusing evidence from the previous shape.
    let modules = &[Q_BASE, Q_NULLABLE];
    let chain = |acquire: &str| {
        format!(
            "void *inner(void) {{ void *p = {acquire}(); return p; }} \
             void *middle(void) {{ void *p = inner(); return p; }} \
             void subject(void) {{ void *p = middle(); if (p) {{ q_free(p); }} }}"
        )
    };

    let nonnull = chain("q_alloc");
    assert_eq!(analyze(&nonnull, modules).status, AnalysisStatus::Safe);
    for function in ["inner", "middle"] {
        assert_eq!(
            summary_in(&nonnull, modules, function),
            ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: false }),
            "{function}"
        );
    }

    // C's acquisition becomes possibly-null: the alternative propagates out
    // through B instead of being collapsed at the first summary application.
    let nullable = chain("q_maybe_alloc");
    assert_eq!(analyze(&nullable, modules).status, AnalysisStatus::Safe);
    for function in ["inner", "middle"] {
        assert_eq!(
            summary_in(&nullable, modules, function),
            ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: true }),
            "{function}"
        );
    }
    assert_ne!(
        analyze(
            &nullable.replace("if (p) { q_free(p); }", "q_free(p);"),
            modules,
        )
        .status,
        AnalysisStatus::Safe
    );

    // C stops being summarizable at all: B loses its summary too, and A's
    // obligation resurfaces rather than resting on the earlier derivation.
    let leaking = "void *inner(void) { void *p = q_alloc(); void *q = q_alloc(); return p; } \
                   void *middle(void) { void *p = inner(); return p; } \
                   void subject(void) { void *p = middle(); q_free(p); }";
    for function in ["inner", "middle"] {
        assert!(no_summary(leaking, &[Q_BASE], function), "{function}");
    }
    assert_ne!(analyze(leaking, &[Q_BASE]).status, AnalysisStatus::Safe);
}

#[test]
fn a_declared_middle_wrapper_governs_its_callers_and_publishes_no_record() {
    // Precedence is not only a leaf property. A declaration on the *middle* of
    // a chain wins at its call sites, the disagreement with inference is
    // reported, and the conflicting inferred summary is withheld — while the
    // inner wrapper's own validated summary is untouched.
    let declared = "module contract; extern fn middle() -> ptr<mut, u8> contract { \
                    requires resource_contract_v1; ensures normal resource_return_none(); };";
    let source = "void *inner(void) { void *p = q_alloc(); return p; } \
                  void *middle(void) { void *p = inner(); return p; } \
                  void subject(void) { void *p = middle(); q_free(p); }";
    let modules = &[Q_BASE, ("middle.qcore", declared)];
    let result = analyze(source, modules);
    assert!(has(&result, "middle", |kind| matches!(
        kind,
        LegacyDiagnosticKind::SummaryConflict { .. }
    )));
    assert_eq!(result.status, AnalysisStatus::Unknown);
    assert!(no_summary(source, modules, "middle"));
    assert_eq!(
        summary_in(source, modules, "inner"),
        ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: false })
    );

    // The declaration, not the inference, is what the environment carries into
    // every call site.
    let tokens = tokenize(source);
    let names = legacy_c_source_functions(source);
    let contracts = fixture_contracts(modules);
    let derived = derive_custody_summaries(&tokens, &names, &contracts);
    assert_eq!(
        derived.environment().contract("middle"),
        contracts.contract("middle")
    );
}

#[test]
fn final_contextual_body_moves_without_losing_summary_or_body_data() {
    let tokens =
        crate::legacy_c::tokenize("void subject(void) { return; } void other(void) { return; }");
    let contracts = ResourceContracts::default();
    let cfg = build_function_cfg(&tokens, "subject", &contracts).unwrap();
    let mut derivation = CustodyDerivation {
        environment: contracts,
        ..CustodyDerivation::default()
    };
    derivation.contextual.insert("subject".into(), cfg.clone());
    derivation.contextual.insert("other".into(), cfg.clone());
    let before = derivation.records();
    derivation.retain_contextual_cfgs(&["subject".to_owned()]);
    assert!(derivation.contextual_cfg("other").is_none());
    let moved = derivation.take_contextual_cfg("subject").unwrap();
    assert_eq!(moved, cfg);
    assert!(derivation.contextual_cfg("subject").is_none());
    assert!(derivation.take_contextual_cfg("subject").is_none());
    assert_eq!(derivation.records(), before);
    assert_eq!(
        analyze_cfg_with_entry(moved, &EntryCustody::default()),
        analyze_cfg_with_entry(cfg, &EntryCustody::default())
    );
}
