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

//! Public-API regression closure for the merged W4 custody-summary
//! subprofiles (card C01, `docs/reports/remaining-c01/README.md`).
//!
//! Every case here attacks the boundary an external consumer actually sees:
//! [`check_legacy_safety_certificate`]. The cardinal expectation is negative —
//! a certificate whose summaries the gateway cannot re-derive from the
//! preprocessed source and the declared contracts alone must be rejected, never
//! accepted on the `InferredValidated` label it carries.
//!
//! Each rejection is asserted against its *reason*, not merely against
//! `is_err()`, so a case cannot pass by failing somewhere else.
//!
//! These tests add no analysis capability and promote no verdict. They pin
//! behavior the merged increments already implement.

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, ContractTrust, LegacySafetyCertificate, ResourceContracts,
    ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode, ResourceSummaryRecord,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const Q_BASE: (&str, &str) = ("q_base.qcore", include_str!("fixtures/contracts/q_base.qv"));
const Q_NULLABLE: (&str, &str) = (
    "q_nullable.qcore",
    include_str!("fixtures/contracts/q_nullable.qv"),
);
const Q_RICH_MODES: (&str, &str) = (
    "q_rich_modes.qcore",
    include_str!("fixtures/contracts/q_rich_modes.qv"),
);
const Q_CONTAINER_STORE: (&str, &str) = (
    "q_container_store.qcore",
    include_str!("fixtures/contracts/q_container_store.qv"),
);
const Q_REALLOC: (&str, &str) = (
    "q_realloc.qcore",
    include_str!("fixtures/contracts/q_realloc.qv"),
);

/// The reason a certificate whose summaries cannot be re-derived is refused.
const SUMMARY_MISMATCH: &str = "inferred custody summaries do not match the preprocessed source";
/// The reason a certificate whose per-function results changed is refused.
const PROGRAM_MISMATCH: &str = "program summary does not match the preprocessed source";

fn contracts(modules: &[(&str, &str)]) -> ResourceContracts {
    load_resource_contract_sources(
        &modules
            .iter()
            .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
            .collect(),
    )
    .unwrap()
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent custody-summary test: {reason}");
            false
        }
    }
}

fn analyze(source: &str, modules: &[(&str, &str)]) -> UnrestrictedLegacyResult {
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
        &contracts(modules),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"))
}

/// A `Safe` certificate that already passed its own independent recheck.
fn certified(source: &str, modules: &[(&str, &str)]) -> LegacySafetyCertificate {
    let result = analyze(source, modules);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("expected a validated custody certificate: {result:?}\n{source}");
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    certificate
}

/// Assert the recheck refuses `certificate` for exactly `reason`. Asserting the
/// reason is what keeps these cases honest: an `is_err()` assertion would also
/// be satisfied by an unrelated failure such as a fixture that no longer parses.
#[track_caller]
fn rejected_because(certificate: &LegacySafetyCertificate, reason: &str, case: &str) {
    match check_legacy_safety_certificate(certificate) {
        Ok(()) => panic!("accepted a tampered certificate: {case}"),
        Err(actual) => assert_eq!(actual, reason, "{case}"),
    }
}

fn record<'a>(
    certificate: &'a mut LegacySafetyCertificate,
    function: &str,
) -> &'a mut ResourceSummaryRecord {
    certificate
        .summaries
        .iter_mut()
        .find(|record| record.function == function)
        .unwrap_or_else(|| panic!("missing validated summary for `{function}`"))
}

fn contract_of(certificate: &LegacySafetyCertificate, function: &str) -> ResourceFunctionContract {
    let record = certificate
        .summaries
        .iter()
        .find(|record| record.function == function)
        .unwrap_or_else(|| panic!("missing validated summary for `{function}`"));
    assert_eq!(record.trust, ContractTrust::InferredValidated);
    record.contract.clone()
}

const CHAIN_DECLARATIONS: &str =
    "void *q_alloc(void); void q_free(void *); void q_observe(void *);";
const CHAIN: &str = "void *inner(void) { void *p = q_alloc(); q_observe(p); return p; } \
                     void *middle(void) { void *p = inner(); return p; } \
                     void subject(void) { void *p = middle(); q_free(p); }";

fn chain_source() -> String {
    format!("{CHAIN_DECLARATIONS}\n{CHAIN}")
}

// --- Acceptance 4: an unvalidated record is rejected, not trusted -----------

#[test]
fn fabricated_validated_summaries_are_rejected_at_the_recheck_boundary() {
    // A `ResourceSummaryRecord` labelled `InferredValidated` is a claim, not
    // evidence. The recheck re-derives the whole vector from the preprocessed
    // source and the declared contracts and compares it, so a record the
    // analyzer would not publish cannot enter through the label.
    if !clang_available() {
        return;
    }
    let source = chain_source();
    let modules = &[Q_BASE, Q_RICH_MODES];
    let certificate = certified(&source, modules);
    assert_eq!(
        contract_of(&certificate, "middle"),
        ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: false })
    );

    // (a) A summary for a function the translation unit does not define.
    let mut forged = certificate.clone();
    forged.summaries.push(ResourceSummaryRecord::new(
        "phantom_release",
        ContractTrust::InferredValidated,
        ResourceFunctionContract::new(
            [ResourceParameterMode::Released { nullable: false }],
            ResourceReturnMode::None,
        ),
    ));
    rejected_because(&forged, SUMMARY_MISMATCH, "added phantom summary");

    // (b) A summary claiming a wrapper releases what it actually hands back.
    let mut forged = certificate.clone();
    record(&mut forged, "middle").contract = ResourceFunctionContract::new(
        [ResourceParameterMode::Released { nullable: false }],
        ResourceReturnMode::None,
    );
    rejected_because(&forged, SUMMARY_MISMATCH, "fabricated release summary");

    // (c) Deleting a record the analysis relied on is equally a mismatch: the
    // certificate must carry exactly what a fresh derivation produces, so a
    // caller cannot hide which summaries its verdict rests on.
    let mut forged = certificate.clone();
    forged.summaries.retain(|record| record.function != "inner");
    rejected_because(&forged, SUMMARY_MISMATCH, "deleted summary");

    // (d) Relabelling a genuine record as a declared assumption. The
    // re-derivation refuses it before the dedicated trust gate is consulted;
    // that gate stays as defense in depth for any future producer.
    let mut forged = certificate.clone();
    record(&mut forged, "inner").trust = ContractTrust::Declared;
    rejected_because(&forged, SUMMARY_MISMATCH, "relabelled trust");
    assert!(
        forged
            .summaries
            .iter()
            .any(|record| record.trust != ContractTrust::InferredValidated)
    );

    // (e) Reattaching a genuine record to another function name.
    let mut forged = certificate.clone();
    record(&mut forged, "inner").function = "phantom_release".to_owned();
    rejected_because(&forged, SUMMARY_MISMATCH, "renamed summary");
}

// --- Acceptance 3: callee mutation re-derives, never reuses -----------------

#[test]
fn a_changed_transitive_callee_body_invalidates_the_whole_certificate() {
    // A calls B calls C. Only C's body changes, and the certificate that was
    // valid for the original source is refused: caller evidence is re-derived
    // from the preprocessed source, never carried across the change.
    if !clang_available() {
        return;
    }
    let modules = &[Q_BASE, Q_RICH_MODES];
    let certificate = certified(&chain_source(), modules);
    for (before, after, reason) in [
        // C now releases what it returns, so C has no summary at all and B's
        // owned result loses its support.
        ("q_observe(p);", "q_free(p);", SUMMARY_MISMATCH),
        // C's acquisition disappears, so C hands back nothing and B's owned
        // result cannot be re-derived.
        (
            "void *p = q_alloc(); q_observe(p);",
            "void *p = 0;",
            SUMMARY_MISMATCH,
        ),
        // C now acquires a second resource it never discharges, in a call
        // nested inside another call's argument.
        //
        // Card O04-B09B10 moved which gate catches this, deliberately and in
        // the fail-closed direction. `find_call` models the *first* call of a
        // statement only, so before that card the nested `q_alloc()` was
        // dropped without a diagnostic — measured directly as a `Safe` verdict
        // on `k_borrow(k_take(1))` — and the mutation surfaced only later, in
        // the per-function results (`PROGRAM_MISMATCH`). A nested call now
        // refuses where it stands, so C publishes no summary at all and the
        // mutation is refused one gate *earlier*. The load-bearing half of this
        // row is unchanged: the mutated source is rejected, never admitted.
        ("q_observe(p);", "q_observe(q_alloc());", SUMMARY_MISMATCH),
    ] {
        let mut changed = certificate.clone();
        changed.preprocessed_source = changed.preprocessed_source.replace(before, after);
        assert_ne!(
            changed.preprocessed_source, certificate.preprocessed_source,
            "fixture text `{before}` not present in the preprocessed source"
        );
        rejected_because(&changed, reason, &format!("`{before}` -> `{after}`"));
    }
    // The `PROGRAM_MISMATCH` gate the third row above used to reach is kept
    // under test directly, rather than left uncovered when that row moved to
    // the earlier gate: a certificate whose *published per-function result*
    // disagrees with the one the recheck rederives from the unchanged source is
    // refused, and refused with that reason.
    let mut forged = certificate.clone();
    forged
        .summary
        .functions
        .get_mut("subject")
        .expect("the entry is summarised")
        .status = AnalysisStatus::Unknown;
    assert_ne!(forged.summary, certificate.summary, "the forgery applied");
    rejected_because(&forged, PROGRAM_MISMATCH, "forged per-function status");
}

#[test]
fn a_changed_transitive_callee_redirects_the_analysis_of_every_caller() {
    // The same shape run forwards: mutating C changes B's published summary and
    // A's obligations together. Nothing is memoized across the two analyses.
    if !clang_available() {
        return;
    }
    let modules = &[Q_BASE, Q_NULLABLE, Q_RICH_MODES];
    let declarations = "void *q_alloc(void); void *q_maybe_alloc(void); void q_free(void *);";
    let chain = |acquire: &str| {
        format!(
            "{declarations}\n\
             void *inner(void) {{ void *p = {acquire}(); return p; }} \
             void *middle(void) {{ void *p = inner(); return p; }} \
             void subject(void) {{ void *p = middle(); if (p) {{ q_free(p); }} }}"
        )
    };

    let nonnull = certified(&chain("q_alloc"), modules);
    for function in ["inner", "middle"] {
        assert_eq!(
            contract_of(&nonnull, function),
            ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: false }),
            "{function}"
        );
    }

    // C's acquisition becomes possibly-null. The alternative is preserved all
    // the way out: both wrappers now publish a nullable owned result rather
    // than collapsing it at the first summary application.
    let nullable = certified(&chain("q_maybe_alloc"), modules);
    for function in ["inner", "middle"] {
        assert_eq!(
            contract_of(&nullable, function),
            ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: true }),
            "{function}"
        );
    }

    // And the obligation the nullable chain hands the caller is a real one: the
    // same caller written without a null test is no longer certified.
    let unchecked = chain("q_maybe_alloc").replace("if (p) { q_free(p); }", "q_free(p);");
    assert!(!matches!(
        analyze(&unchecked, modules),
        UnrestrictedLegacyResult::Safe(_)
    ));
}

// --- Acceptance 1: alternatives and owner identity survive application ------

#[test]
fn a_possibly_null_owned_result_cannot_be_collapsed_to_non_null() {
    // Collapsing `nullable` would delete the caller's null-check obligation,
    // which is the whole content of the summary. Both directions are refused,
    // so the two alternatives are distinguished rather than merely ordered.
    if !clang_available() {
        return;
    }
    let modules = &[Q_BASE, Q_NULLABLE];
    let source = "void *q_maybe_alloc(void); void q_free(void *); \
                  void *make(void) { void *p = q_maybe_alloc(); return p; } \
                  void subject(void) { void *p = make(); if (p) { q_free(p); } }";
    let certificate = certified(source, modules);
    assert_eq!(
        contract_of(&certificate, "make"),
        ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: true })
    );
    for result in [
        ResourceReturnMode::Owned { nullable: false },
        ResourceReturnMode::None,
    ] {
        let mut forged = certificate.clone();
        record(&mut forged, "make").contract.result = result;
        rejected_because(&forged, SUMMARY_MISMATCH, &format!("{result:?}"));
    }

    // The non-null acquisition is the control: claiming `nullable` there is
    // equally a mismatch.
    let nonnull = certified(&source.replace("q_maybe_alloc", "q_alloc"), modules);
    assert_eq!(
        contract_of(&nonnull, "make"),
        ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: false })
    );
    let mut forged = nonnull.clone();
    record(&mut forged, "make").contract.result = ResourceReturnMode::Owned { nullable: true };
    rejected_because(&forged, SUMMARY_MISMATCH, "widened a non-null result");
}

#[test]
fn a_nullable_out_slot_cannot_be_collapsed_to_a_guaranteed_one() {
    // The same alternative on the out-slot axis: the failure path stores a null
    // constant, and a summary that hid it would promise the caller a resource
    // that path never produced.
    if !clang_available() {
        return;
    }
    let modules = &[Q_BASE, Q_RICH_MODES];
    let source = "void *q_alloc(void); void q_free(void *); \
                  int fill(void **out, int c) { if (c) { void *p = q_alloc(); *out = p; return 0; } *out = 0; return -1; } \
                  void subject(void) { void *p; int rc = fill(&p, 1); if (p) { q_free(p); } }";
    let certificate = certified(source, modules);
    assert_eq!(
        contract_of(&certificate, "fill").parameters[0],
        ResourceParameterMode::Out { nullable: true }
    );
    for mode in [
        ResourceParameterMode::Out { nullable: false },
        ResourceParameterMode::Borrowed,
        ResourceParameterMode::Owned { nullable: false },
    ] {
        let mut forged = certificate.clone();
        record(&mut forged, "fill").contract.parameters[0] = mode;
        rejected_because(&forged, SUMMARY_MISMATCH, &format!("{mode:?}"));
    }
}

#[test]
fn stored_owner_identity_survives_certificate_rederivation() {
    // `Stored` names *which* caller-held formal now owns the item. A summary
    // that keeps the mode but moves the owner describes a different transfer,
    // and one that drops the mode returns custody the caller no longer has.
    if !clang_available() {
        return;
    }
    let modules = &[Q_BASE, Q_CONTAINER_STORE];
    let source = "void *q_alloc(void); void q_free(void *); void q_store(void *, void *); \
                  void add(void *owner, void *item) { q_store(owner, item); } \
                  void subject(void) { void *c = q_alloc(); void *i = q_alloc(); add(c, i); q_free(c); }";
    let certificate = certified(source, modules);
    assert_eq!(
        contract_of(&certificate, "add"),
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
    for mode in [
        ResourceParameterMode::Stored {
            owner_parameter: 1,
            nullable: false,
        },
        ResourceParameterMode::Stored {
            owner_parameter: 0,
            nullable: true,
        },
        ResourceParameterMode::Owned { nullable: false },
        ResourceParameterMode::Released { nullable: false },
        ResourceParameterMode::Consumed { nullable: false },
    ] {
        let mut forged = certificate.clone();
        record(&mut forged, "add").contract.parameters[1] = mode;
        rejected_because(&forged, SUMMARY_MISMATCH, &format!("{mode:?}"));
    }
}

#[test]
fn reallocated_result_identity_and_its_null_alternative_are_rechecked() {
    // `Reallocated` names the formal whose custody the result decides, and it
    // is meaningful only while the result may be null: that null test is the
    // only thing telling the caller whether the old pointer survived. Moving
    // the index, dropping the alternative, or restating it as a plain owned
    // result each describe a different call.
    if !clang_available() {
        return;
    }
    let modules = &[Q_BASE, Q_REALLOC];
    let source = "void *q_alloc(void); void q_free(void *); void *q_realloc(void *, __SIZE_TYPE__); \
                  void *grow(void *p, __SIZE_TYPE__ n) { void *q = q_realloc(p, n); return q; } \
                  void subject(void) { void *p = q_alloc(); void *q = grow(p, 8); if (q) { q_free(q); } else { q_free(p); } }";
    let certificate = certified(source, modules);
    assert_eq!(
        contract_of(&certificate, "grow"),
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
    for result in [
        ResourceReturnMode::Reallocated {
            parameter: 1,
            nullable: true,
        },
        ResourceReturnMode::Reallocated {
            parameter: 0,
            nullable: false,
        },
        ResourceReturnMode::Owned { nullable: true },
        ResourceReturnMode::None,
    ] {
        let mut forged = certificate.clone();
        record(&mut forged, "grow").contract.result = result;
        rejected_because(&forged, SUMMARY_MISMATCH, &format!("{result:?}"));
    }
}
