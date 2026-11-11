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

//! LOADER-ABI-MODES — a declared custody *mode* is compared against the
//! validated summary derived from the callee's own body
//! (`docs/reports/remaining-loader-abi-modes/README.md`).
//!
//! Card LOADER-ABI closed the *type* half of this and recorded the rest as its
//! own follow-on: "this card compares TYPES; nothing compares a declared mode
//! against anything the source says". A type-faithful declaration with an
//! unfaithful mode was therefore exactly as trusted as a faithful one, and §1
//! below reproduces three `Safe` callers that rest on one.
//!
//! **The family is held out.** An `m_*` API unrelated to Fluent Bit, ChunkIO,
//! Monkey, libevent and every earlier card's fixtures.
//!
//! Nothing here promotes anything: every `Safe` below is `Safe` for a synthetic
//! declared custody projection under the premises the fixture names, and every
//! provisional `Unsafe` is a projection allegation, never an upstream defect
//! claim (hard invariant 2).

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, ModeOutcome, ModeRefusal, ResourceContracts, ResourceFunctionContract,
    ResourceParameterMode, ResourceReturnMode, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    declared_mode_census, mode_contradiction,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::reason::{ReasonCategory, ReasonSubject};

fn load(source: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "m_api.qcore".to_owned(),
        format!("module contract;\n\n{source}"),
    )]))
    .expect("the fixture module loads")
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .expect("the function is analysed")
}

fn unsupported(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect()
}

fn has_summary_conflict(summary: &LegacyFunctionSummary) -> bool {
    summary.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.kind,
            LegacyDiagnosticKind::SummaryConflict { .. }
        )
    })
}

fn analyse(source: &str, contracts: &ResourceContracts) -> LegacyProgramSummary {
    qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit(source, "caller", contracts)
        .expect("the fixture tokenizes")
}

/// The refusal a mode-withdrawn callee's call carries, by key.
fn withdrawal(callee: &str, refusal: ModeRefusal) -> String {
    format!(
        "call to `{callee}` whose declared contract was withdrawn against the summary derived \
         from its body in this translation unit ({})",
        refusal.as_str()
    )
}

fn outcomes(source: &str, contracts: &ResourceContracts) -> BTreeMap<String, ModeOutcome> {
    declared_mode_census(source, contracts)
        .into_iter()
        .map(|row| (row.function, row.outcome))
        .collect()
}

fn clang_available() -> bool {
    qcore_c_gateway::clang_command().is_ok()
}

/// The allocator and releaser every fixture below shares. Both are declared
/// faithfully and neither is *defined* in any fixture, so this card never
/// compares them: they are the ambient projection, not the subject.
const AMBIENT: &str = "\
extern fn m_alloc(size: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_return_owned(nullable);
};

extern fn m_free(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
};
";

#[cfg(all(windows, target_pointer_width = "64"))]
const AMBIENT_C: &str = "\
typedef unsigned long long size_t;
void *m_alloc(size_t size);
void m_free(void *p);
";

#[cfg(all(not(windows), target_pointer_width = "64"))]
const AMBIENT_C: &str = "\
typedef unsigned long size_t;
void *m_alloc(size_t size);
void m_free(void *p);
";

#[cfg(target_pointer_width = "32")]
const AMBIENT_C: &str = "\
typedef unsigned int size_t;
void *m_alloc(size_t size);
void m_free(void *p);
";

// ---------------------------------------------------------------------------
// 1. The fail-open, reproduced in all three classes, and closed
// ---------------------------------------------------------------------------

/// **Class 1 — a declared `Borrowed` the body releases.** The caller keeps its
/// duty across a call that already destroyed the resource, so it releases it a
/// second time and the projection calls that `Safe`.
///
/// Measured before this card: `caller` **`Safe`**, with *no* diagnostic at all,
/// beside an `m_sink` already reported `Unknown` with a `SummaryConflict`. The
/// disagreement was known and the caller was checked against it anyway.
#[test]
fn a_borrowed_parameter_the_body_releases_is_withdrawn() {
    let contracts = load(&format!(
        "{AMBIENT}
extern fn m_sink(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
}};
"
    ));
    let source = format!(
        "{AMBIENT_C}\
void m_sink(void *p);
void m_sink(void *p) {{ m_free(p); }}
void caller(void) {{ void *h = m_alloc(8); m_sink(h); m_free(h); }}
"
    );
    let summary = analyse(&source, &contracts);
    let caller = function(&summary, "caller");
    assert_ne!(
        caller.status,
        AnalysisStatus::Safe,
        "the double release must no longer be certified: {:?}",
        unsupported(caller)
    );
    assert!(
        unsupported(caller).contains(&withdrawal(
            "m_sink",
            ModeRefusal::ParameterCustodyContradiction
        )),
        "{:?}",
        unsupported(caller)
    );
    assert_eq!(
        outcomes(&source, &contracts).get("m_sink"),
        Some(&ModeOutcome::Contradicted(
            ModeRefusal::ParameterCustodyContradiction
        ))
    );
}

/// **Class 2 — a declared `Released` the body never performs.** The caller
/// drops its duty at a call that left the resource live, so the leak is
/// invisible.
///
/// The withdrawal makes the leak visible, which is a **provisional `Unsafe`**:
/// the caller now holds an acquisition nothing accounts for. That is the
/// direction card LOADER-ABI recorded as `LOADER-ABI-WITHDRAWAL-UNSAFE`, it is
/// asserted here rather than left to surprise a later reader, and it is a
/// statement about this synthetic fixture's projection — not an upstream defect.
#[test]
fn a_released_parameter_the_body_never_releases_is_withdrawn() {
    let contracts = load(&format!(
        "{AMBIENT}
extern fn m_retire(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
}};
"
    ));
    let source = format!(
        "{AMBIENT_C}\
void m_retire(void *p);
void m_retire(void *p) {{ (void)p; }}
void caller(void) {{ void *h = m_alloc(8); m_retire(h); }}
"
    );
    let summary = analyse(&source, &contracts);
    let caller = function(&summary, "caller");
    // The declaration's hidden leak used to be reported here (`Unsafe`). Card
    // W44-LEAK-PIN-AUDIT: the withdrawn call site is a refused statement that
    // hands `h` to `m_retire`, so the refused hand-on rule withdraws the leak;
    // the caller is `Unknown`, never `Safe`, and the withdrawal is reported.
    assert_eq!(caller.status, AnalysisStatus::Unknown);
    assert!(
        !caller
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "a leak through a refused hand-on is not claimed: {:?}",
        caller.diagnostics
    );
    assert!(
        unsupported(caller).contains(&withdrawal(
            "m_retire",
            ModeRefusal::ParameterCustodyContradiction
        )),
        "the withdrawal is reported, so the allegation is never silent: {:?}",
        unsupported(caller)
    );
}

/// **Class 3 — a declared absent result the body produces.** The caller
/// accounts for nothing while the body hands back fresh custody.
#[test]
fn an_absent_result_the_body_produces_is_withdrawn() {
    let contracts = load(&format!(
        "{AMBIENT}
extern fn m_make() -> ptr<mut, u8>
contract {{
    requires resource_contract_v1;
    ensures normal resource_return_none();
}};
"
    ));
    let source = format!(
        "{AMBIENT_C}\
void *m_make(void);
void *m_make(void) {{ void *p = m_alloc(8); return p; }}
void caller(void) {{ m_make(); }}
"
    );
    let summary = analyse(&source, &contracts);
    let caller = function(&summary, "caller");
    assert_ne!(caller.status, AnalysisStatus::Safe);
    assert_eq!(
        unsupported(caller),
        vec![withdrawal(
            "m_make",
            ModeRefusal::ResultCustodyContradiction
        )]
    );
    assert_eq!(
        outcomes(&source, &contracts).get("m_make"),
        Some(&ModeOutcome::Contradicted(
            ModeRefusal::ResultCustodyContradiction
        ))
    );
}

// ---------------------------------------------------------------------------
// 2. Non-vacuity: the gate is not refusing everything
// ---------------------------------------------------------------------------

/// The same three bodies under **faithful** declarations. Nothing is withdrawn,
/// every declaration is `Agreed`, and the caller decides on its own merits.
///
/// Without this row every test above could be passing because the gate refuses
/// any contracted callee that happens to be defined in the unit.
#[test]
fn faithful_declarations_are_admitted_and_still_decide() {
    let contracts = load(&format!(
        "{AMBIENT}
extern fn m_sink(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_released(p, nonnull);
    ensures normal resource_return_none();
}};
"
    ));
    let source = format!(
        "{AMBIENT_C}\
void m_sink(void *p);
void m_sink(void *p) {{ m_free(p); }}
void caller(void) {{ void *h = m_alloc(8); if (h) {{ m_sink(h); }} }}
"
    );
    let summary = analyse(&source, &contracts);
    let caller = function(&summary, "caller");
    assert_eq!(
        caller.status,
        AnalysisStatus::Safe,
        "the faithful declaration must still certify: {:?}",
        caller.diagnostics
    );
    assert!(unsupported(caller).is_empty());
    assert_eq!(
        outcomes(&source, &contracts).get("m_sink"),
        Some(&ModeOutcome::Agreed)
    );
    assert!(
        !has_summary_conflict(function(&summary, "m_sink")),
        "an agreeing declaration reports no conflict either"
    );
}

/// **Absence of evidence is not evidence.** A declared contract whose callee
/// this unit does not define is never compared, and applies exactly as it did
/// before this card.
///
/// This is the half that keeps the declared-trust architecture: requiring a
/// derived summary before applying a contract would withdraw every declaration
/// in the corpus, because the bodies live in other translation units.
#[test]
fn a_callee_defined_in_another_unit_is_never_compared() {
    let contracts = load(&format!(
        "{AMBIENT}
extern fn m_sink(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_released(p, nonnull);
    ensures normal resource_return_none();
}};
"
    ));
    let source = format!(
        "{AMBIENT_C}\
void m_sink(void *p);
void caller(void) {{ void *h = m_alloc(8); if (h) {{ m_sink(h); }} }}
"
    );
    let summary = analyse(&source, &contracts);
    assert_eq!(function(&summary, "caller").status, AnalysisStatus::Safe);
    assert_eq!(
        outcomes(&source, &contracts),
        BTreeMap::from([
            ("m_alloc".to_owned(), ModeOutcome::Undefined),
            ("m_free".to_owned(), ModeOutcome::Undefined),
            ("m_sink".to_owned(), ModeOutcome::Undefined),
        ])
    );

    // The census short-circuits a unit that defines no contracted name, and the
    // short-circuit is exact: adding a definition of one — with a *faithful*
    // declaration, so nothing is withdrawn — takes the full route and every
    // other row is unchanged. Without this half the row above could be passing
    // because the short-circuit answers `Undefined` for everything.
    let defining = format!(
        "{AMBIENT_C}\
void m_sink(void *p);
void m_sink(void *p) {{ m_free(p); }}
void caller(void) {{ void *h = m_alloc(8); if (h) {{ m_sink(h); }} }}
"
    );
    assert_eq!(
        outcomes(&defining, &contracts),
        BTreeMap::from([
            ("m_alloc".to_owned(), ModeOutcome::Undefined),
            ("m_free".to_owned(), ModeOutcome::Undefined),
            ("m_sink".to_owned(), ModeOutcome::Agreed),
        ])
    );
}

/// A body the projection cannot summarise yields **no** derived summary, so
/// there is nothing to contradict and the declaration still applies.
///
/// Here `m_sink` releases on one path and not the other: no single parameter
/// mode describes every exit, so W4 publishes nothing. The declaration — which
/// happens to be the one a caller wants — is believed, exactly as before.
#[test]
fn a_body_that_yields_no_summary_leaves_the_declaration_in_force() {
    let contracts = load(&format!(
        "{AMBIENT}
extern fn m_sink(p: ptr<mut, u8>, keep: i32) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_released(p, nonnull);
    ensures normal resource_borrowed(keep);
    ensures normal resource_return_none();
}};
"
    ));
    let source = format!(
        "{AMBIENT_C}\
void m_sink(void *p, int keep);
void m_sink(void *p, int keep) {{ if (keep) {{ return; }} m_free(p); }}
void caller(void) {{ void *h = m_alloc(8); if (h) {{ m_sink(h, 0); }} }}
"
    );
    let summary = analyse(&source, &contracts);
    assert_eq!(
        function(&summary, "caller").status,
        AnalysisStatus::Safe,
        "an uncheckable declaration keeps the trusted premise it already was"
    );
    assert_eq!(
        outcomes(&source, &contracts).get("m_sink"),
        Some(&ModeOutcome::NoSummary)
    );
}

// ---------------------------------------------------------------------------
// 3. The duty relation, decided row by row
// ---------------------------------------------------------------------------

/// Every representative of every custody duty, in both positions.
///
/// The relation is taken in *duties*, not in modes: a difference has to be one
/// a caller can be wrong about — which side holds the resource after the call.
/// The table below is the whole relation, and it is decided exhaustively rather
/// than sampled.
#[test]
fn the_duty_relation_decides_every_pair() {
    // (label, mode, duty group). Modes in the same group must never contradict
    // each other; modes in different groups always must.
    let modes: [(&str, ResourceParameterMode, &str); 8] = [
        ("borrowed", ResourceParameterMode::Borrowed, "retained"),
        (
            "owned",
            ResourceParameterMode::Owned { nullable: false },
            "retained",
        ),
        (
            "released",
            ResourceParameterMode::Released { nullable: false },
            "transferred",
        ),
        (
            "consumed",
            ResourceParameterMode::Consumed { nullable: false },
            "transferred",
        ),
        (
            "stored",
            ResourceParameterMode::Stored {
                owner_parameter: 1,
                nullable: false,
            },
            "transferred",
        ),
        (
            "out",
            ResourceParameterMode::Out { nullable: false },
            "delivered",
        ),
        (
            "removed",
            ResourceParameterMode::Removed {
                owner_parameter: 1,
                nullable: false,
            },
            "restored",
        ),
        (
            "owned-nullable",
            ResourceParameterMode::Owned { nullable: true },
            "retained",
        ),
    ];
    for (declared_label, declared_mode, declared_duty) in modes {
        for (derived_label, derived_mode, derived_duty) in modes {
            let declared = ResourceFunctionContract::new(
                [declared_mode, ResourceParameterMode::Borrowed],
                ResourceReturnMode::None,
            );
            let derived = ResourceFunctionContract::new(
                [derived_mode, ResourceParameterMode::Borrowed],
                ResourceReturnMode::None,
            );
            let expected = (declared_duty != derived_duty)
                .then_some(ModeRefusal::ParameterCustodyContradiction);
            assert_eq!(
                mode_contradiction(&declared, &derived),
                expected,
                "declared {declared_label} against derived {derived_label}"
            );
        }
    }

    // The result axis, likewise. `Reallocated` produces custody just as `Owned`
    // does, so the two never contradict each other; either contradicts `None`.
    let results: [(&str, ResourceReturnMode, bool); 4] = [
        ("none", ResourceReturnMode::None, false),
        ("owned", ResourceReturnMode::Owned { nullable: false }, true),
        (
            "owned-nullable",
            ResourceReturnMode::Owned { nullable: true },
            true,
        ),
        (
            "reallocated",
            ResourceReturnMode::Reallocated {
                parameter: 0,
                nullable: false,
            },
            true,
        ),
    ];
    for (declared_label, declared_result, declared_produces) in results {
        for (derived_label, derived_result, derived_produces) in results {
            let declared = ResourceFunctionContract::new([], declared_result);
            let derived = ResourceFunctionContract::new([], derived_result);
            let expected = (declared_produces != derived_produces)
                .then_some(ModeRefusal::ResultCustodyContradiction);
            assert_eq!(
                mode_contradiction(&declared, &derived),
                expected,
                "declared result {declared_label} against derived {derived_label}"
            );
        }
    }
}

/// **The two-sided tie.** The same finite duty domain, in the same order, is
/// decided in Lean (`QCore.DeclaredModeAgreement.the_checksum_is_pinned`) and
/// here, and reduced to one positional checksum. A permutation moves it and a
/// single flipped row moves it, so the two decisions cannot drift apart
/// silently.
#[test]
fn the_duty_domain_checksum_matches_the_lean_decision() {
    // `parameterDomain`, in the Lean module's order.
    let parameters = [
        ResourceParameterMode::Borrowed,
        ResourceParameterMode::Owned { nullable: false },
        ResourceParameterMode::Released { nullable: false },
        ResourceParameterMode::Consumed { nullable: false },
        ResourceParameterMode::Stored {
            owner_parameter: 1,
            nullable: false,
        },
        ResourceParameterMode::Out { nullable: false },
        ResourceParameterMode::Removed {
            owner_parameter: 1,
            nullable: false,
        },
        ResourceParameterMode::Owned { nullable: true },
    ];
    // `resultDomain`, likewise.
    let results = [
        ResourceReturnMode::None,
        ResourceReturnMode::Owned { nullable: false },
        ResourceReturnMode::Owned { nullable: true },
        ResourceReturnMode::Reallocated {
            parameter: 0,
            nullable: false,
        },
    ];
    let mut parameter_verdicts = Vec::new();
    for declared_mode in parameters {
        for derived_mode in parameters {
            let declared = ResourceFunctionContract::new(
                [declared_mode, ResourceParameterMode::Borrowed],
                ResourceReturnMode::None,
            );
            let derived = ResourceFunctionContract::new(
                [derived_mode, ResourceParameterMode::Borrowed],
                ResourceReturnMode::None,
            );
            parameter_verdicts.push(usize::from(
                mode_contradiction(&declared, &derived).is_some(),
            ));
        }
    }
    let mut result_verdicts = Vec::new();
    for declared_result in results {
        for derived_result in results {
            let declared = ResourceFunctionContract::new([], declared_result);
            let derived = ResourceFunctionContract::new([], derived_result);
            result_verdicts.push(usize::from(
                mode_contradiction(&declared, &derived).is_some(),
            ));
        }
    }
    assert_eq!(parameter_verdicts.len(), 64);
    assert_eq!(result_verdicts.len(), 16);
    let weigh = |verdicts: &[usize]| -> usize {
        verdicts
            .iter()
            .enumerate()
            .map(|(index, verdict)| verdict * (index + 1))
            .sum()
    };
    assert_eq!(weigh(&parameter_verdicts), 1502);
    assert_eq!(weigh(&result_verdicts), 36);
    assert_eq!(
        weigh(&parameter_verdicts) + 1000 * weigh(&result_verdicts),
        1502 + 1000 * 36,
        "the Lean namespace decides the same domain to the same checksum"
    );
}

/// A refinement inside one duty is **not** a contradiction, in either
/// direction: nullability, a `Stored`/`Removed` owner index and a
/// `Reallocated` replaced parameter change what the caller must additionally
/// prove, never which side holds the resource after the call.
///
/// The existing `SummaryConflict` still reports them, so nothing is lost — this
/// is where the card deliberately stops (`LOADER-ABI-MODES-REFINEMENT`).
#[test]
fn a_refinement_inside_one_duty_is_not_a_contradiction() {
    let pairs = [
        (
            ResourceParameterMode::Released { nullable: true },
            ResourceParameterMode::Released { nullable: false },
        ),
        (
            ResourceParameterMode::Stored {
                owner_parameter: 0,
                nullable: false,
            },
            ResourceParameterMode::Stored {
                owner_parameter: 1,
                nullable: false,
            },
        ),
        (
            ResourceParameterMode::Removed {
                owner_parameter: 0,
                nullable: true,
            },
            ResourceParameterMode::Removed {
                owner_parameter: 1,
                nullable: false,
            },
        ),
    ];
    for (declared_mode, derived_mode) in pairs {
        let declared = ResourceFunctionContract::new([declared_mode], ResourceReturnMode::None);
        let derived = ResourceFunctionContract::new([derived_mode], ResourceReturnMode::None);
        assert_ne!(declared, derived, "the pair is a real disagreement");
        assert_eq!(mode_contradiction(&declared, &derived), None);
    }
    assert_eq!(
        mode_contradiction(
            &ResourceFunctionContract::new([], ResourceReturnMode::Owned { nullable: false }),
            &ResourceFunctionContract::new(
                [],
                ResourceReturnMode::Reallocated {
                    parameter: 3,
                    nullable: true
                }
            ),
        ),
        None
    );
}

/// End to end: a nullability refinement is reported as a `SummaryConflict` and
/// **not** withdrawn, so the census reads `Disagreed` and the declaration goes
/// on governing call sites.
#[test]
fn a_nullability_refinement_is_reported_and_not_withdrawn() {
    let contracts = load(&format!(
        "{AMBIENT}
extern fn m_sink(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
}};
"
    ));
    let source = format!(
        "{AMBIENT_C}\
void m_sink(void *p);
void m_sink(void *p) {{ m_free(p); }}
void caller(void) {{ void *h = m_alloc(8); m_sink(h); }}
"
    );
    let summary = analyse(&source, &contracts);
    assert_eq!(
        outcomes(&source, &contracts).get("m_sink"),
        Some(&ModeOutcome::Disagreed)
    );
    assert!(
        has_summary_conflict(function(&summary, "m_sink")),
        "the disagreement is still reported at the definition"
    );
    assert!(
        !unsupported(function(&summary, "caller"))
            .iter()
            .any(|text| text.contains("withdrawn against the summary")),
        "a refinement withdraws nothing: {:?}",
        unsupported(function(&summary, "caller"))
    );
}

/// A parameter vector the derivation read at a different length decides
/// nothing here. That is a statement about the *signature*, which is card
/// LOADER-ABI's gate — and for a function this unit defines it is already
/// decided there, a definition being a declaration.
#[test]
fn an_arity_difference_decides_nothing_on_this_axis() {
    let declared = ResourceFunctionContract::new(
        [ResourceParameterMode::Released { nullable: false }],
        ResourceReturnMode::None,
    );
    let derived = ResourceFunctionContract::new(
        [
            ResourceParameterMode::Borrowed,
            ResourceParameterMode::Borrowed,
        ],
        ResourceReturnMode::None,
    );
    assert_eq!(mode_contradiction(&declared, &derived), None);
    // …but a result contradiction at a differing arity is still decided, so the
    // arity rule narrows one axis rather than switching the gate off.
    let derived = ResourceFunctionContract::new(
        [
            ResourceParameterMode::Borrowed,
            ResourceParameterMode::Borrowed,
        ],
        ResourceReturnMode::Owned { nullable: false },
    );
    assert_eq!(
        mode_contradiction(&declared, &derived),
        Some(ModeRefusal::ResultCustodyContradiction)
    );
}

// ---------------------------------------------------------------------------
// 4. What a withdrawal can and cannot do
// ---------------------------------------------------------------------------

/// A withdrawal is monotone in the refusing direction: it can only replace a
/// modeled call with an uncontracted one, which is `Unknown` and poisons the
/// enclosing function. No verdict can move towards `Safe`.
///
/// Measured on the same body under the contradicted declaration and under no
/// declaration at all — the two states the withdrawal sits between.
#[test]
fn a_withdrawal_never_moves_a_verdict_towards_safe() {
    let body = format!(
        "{AMBIENT_C}\
void m_sink(void *p);
void m_sink(void *p) {{ m_free(p); }}
void caller(void) {{ void *h = m_alloc(8); m_sink(h); m_free(h); }}
"
    );
    let contradicted = load(&format!(
        "{AMBIENT}
extern fn m_sink(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
}};
"
    ));
    let withdrawn = analyse(&body, &contradicted);
    assert_ne!(function(&withdrawn, "caller").status, AnalysisStatus::Safe);
    // The same call with nothing declared for `m_sink` at all: `Unknown` too.
    // A withdrawal lands here, never anywhere safer.
    let undeclared = analyse(&body, &load(AMBIENT));
    assert_ne!(function(&undeclared, "caller").status, AnalysisStatus::Safe);
}

/// The environment is **not** reduced by a withdrawal, and that is the design
/// point: the declared contract is what makes the derivation record the
/// conflict, so removing it would delete the `SummaryConflict` and the
/// `Unknown` it forces on the defining function.
///
/// Both halves are asserted together, so a future change cannot quietly trade
/// one for the other.
#[test]
fn the_defining_function_keeps_its_conflict_beside_the_withdrawal() {
    let contracts = load(&format!(
        "{AMBIENT}
extern fn m_sink(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
}};
"
    ));
    let source = format!(
        "{AMBIENT_C}\
void m_sink(void *p);
void m_sink(void *p) {{ m_free(p); }}
void caller(void) {{ void *h = m_alloc(8); m_sink(h); }}
"
    );
    let summary = analyse(&source, &contracts);
    let definition = function(&summary, "m_sink");
    assert!(
        has_summary_conflict(definition),
        "the definition still reports the conflict: {:?}",
        definition.diagnostics
    );
    assert_eq!(definition.status, AnalysisStatus::Unknown);
    assert!(
        unsupported(function(&summary, "caller")).contains(&withdrawal(
            "m_sink",
            ModeRefusal::ParameterCustodyContradiction
        )),
        "and the caller now refuses too"
    );
}

/// **The carried set is load-bearing, and this is the case that shows it.**
///
/// `m_inner` is declared `Borrowed` and frees; `m_outer` is declared `Released`
/// and only calls `m_inner`. The *first* round convicts both — `m_outer`'s
/// summary reads `Borrowed` because `m_inner`'s declaration says the call keeps
/// the resource. The *second* round convicts neither: `m_inner`'s call inside
/// `m_outer` is now an uncontracted call, so `m_outer` yields no summary at all.
///
/// Recomputing the decision each round would therefore acquit `m_outer` and
/// oscillate. Carrying the accumulated set keeps both withdrawn, which is the
/// refusing direction; both halves are asserted here.
#[test]
fn the_withdrawn_set_is_carried_rather_than_recomputed() {
    let contracts = load(&format!(
        "{AMBIENT}
extern fn m_inner(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
}};

extern fn m_outer(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
}};
"
    ));
    let source = format!(
        "{AMBIENT_C}\
void m_inner(void *p);
void m_outer(void *p);
void m_inner(void *p) {{ m_free(p); }}
void m_outer(void *p) {{ m_inner(p); }}
void caller(void) {{ void *h = m_alloc(8); m_outer(h); }}
"
    );
    let measured = outcomes(&source, &contracts);
    assert_eq!(
        measured.get("m_inner"),
        Some(&ModeOutcome::Contradicted(
            ModeRefusal::ParameterCustodyContradiction
        ))
    );
    assert_eq!(
        measured.get("m_outer"),
        Some(&ModeOutcome::Contradicted(
            ModeRefusal::ParameterCustodyContradiction
        )),
        "the outer declaration stays withdrawn although the settled round can no \
         longer derive the summary that convicted it"
    );
    let summary = analyse(&source, &contracts);
    assert!(
        unsupported(function(&summary, "caller")).contains(&withdrawal(
            "m_outer",
            ModeRefusal::ParameterCustodyContradiction
        ))
    );
    assert!(
        unsupported(function(&summary, "m_outer")).contains(&withdrawal(
            "m_inner",
            ModeRefusal::ParameterCustodyContradiction
        )),
        "the inner withdrawal reaches the outer body too: {:?}",
        unsupported(function(&summary, "m_outer"))
    );
}

/// **The contradiction is relative to the environment's other premises**, and
/// that boundary is measured rather than argued (`LOADER-ABI-MODES-RELATIVE`).
///
/// `m_outer` is declared `Borrowed` and its body only calls `m_inner`, whose own
/// declaration — the one this gate convicts — says the call keeps the resource.
/// Under that premise `m_outer`'s declaration *agrees*, so `m_outer` is not
/// convicted; once `m_inner` is withdrawn, `m_outer` yields no summary and its
/// declaration goes on applying. A body's faithfulness is only ever decided
/// against the interfaces its own unit declares.
#[test]
fn a_contradiction_is_relative_to_the_environments_other_premises() {
    let contracts = load(&format!(
        "{AMBIENT}
extern fn m_inner(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
}};

extern fn m_outer(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
}};
"
    ));
    let source = format!(
        "{AMBIENT_C}\
void m_inner(void *p);
void m_outer(void *p);
void m_inner(void *p) {{ m_free(p); }}
void m_outer(void *p) {{ m_inner(p); }}
void caller(void) {{ void *h = m_alloc(8); m_outer(h); }}
"
    );
    let measured = outcomes(&source, &contracts);
    assert_eq!(
        measured.get("m_inner"),
        Some(&ModeOutcome::Contradicted(
            ModeRefusal::ParameterCustodyContradiction
        )),
        "the directly measurable body is convicted"
    );
    assert_eq!(
        measured.get("m_outer"),
        Some(&ModeOutcome::NoSummary),
        "the transitively unfaithful one is not: nothing here derives through a \
         withdrawn interface"
    );
}

/// **A withdrawn function's own recursive call is exempt from the rewrite**,
/// and both halves of why are asserted together.
///
/// Rewriting it would change the body the derivation reads, so the very summary
/// that convicted the declaration would stop being derivable — taking the
/// `SummaryConflict` and the `Unknown` it forces on the defining function with
/// it. A fail-closed pin from card C03-NFR measured exactly that during
/// implementation and was not weakened; this row keeps the repair honest.
///
/// The exemption costs nothing the withdrawal exists to buy: every genuine
/// caller is still rewritten.
#[test]
fn a_recursive_self_call_is_exempt_so_the_conflict_survives() {
    let contracts = load(&format!(
        "{AMBIENT}
extern fn m_make(n: i32) -> ptr<mut, u8>
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
}};
"
    ));
    let source = format!(
        "{AMBIENT_C}\
void *m_make(int n);
void *m_make(int n) {{ if (n > 0) {{ void *r = m_make(n - 1); return r; }} void *f = m_alloc(8); return f; }}
void caller(void) {{ m_make(2); }}
"
    );
    let summary = analyse(&source, &contracts);
    let definition = function(&summary, "m_make");
    assert!(
        has_summary_conflict(definition),
        "the self-recursive definition keeps the conflict that convicted it: {:?}",
        definition.diagnostics
    );
    assert_eq!(definition.status, AnalysisStatus::Unknown);
    assert!(
        !unsupported(definition)
            .iter()
            .any(|text| text.contains("withdrawn against the summary")),
        "its own recursive call is not rewritten: {:?}",
        unsupported(definition)
    );
    assert!(
        unsupported(function(&summary, "caller")).contains(&withdrawal(
            "m_make",
            ModeRefusal::ResultCustodyContradiction
        )),
        "…while a genuine caller still refuses: {:?}",
        unsupported(function(&summary, "caller"))
    );
    assert_eq!(
        outcomes(&source, &contracts).get("m_make"),
        Some(&ModeOutcome::Contradicted(
            ModeRefusal::ResultCustodyContradiction
        ))
    );
}

/// The definition of a withdrawn callee is not itself renamed: the rewrite is
/// at call positions only, so the function this unit defines is still analysed
/// and still reported under its own name.
#[test]
fn a_withdrawal_renames_call_sites_and_not_the_definition() {
    let contracts = load(&format!(
        "{AMBIENT}
extern fn m_sink(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
}};
"
    ));
    let source = format!(
        "{AMBIENT_C}\
void m_sink(void *p);
void m_sink(void *p) {{ m_free(p); }}
void caller(void) {{ void *h = m_alloc(8); m_sink(h); }}
"
    );
    let summary = analyse(&source, &contracts);
    assert_eq!(
        summary.functions.keys().cloned().collect::<BTreeSet<_>>(),
        BTreeSet::from(["caller".to_owned(), "m_sink".to_owned()])
    );
}

// ---------------------------------------------------------------------------
// 5. The vocabulary, and composition with card LOADER-ABI
// ---------------------------------------------------------------------------

/// The keyed refusal vocabulary is closed, round-trips through the token a
/// withdrawn call site carries, and is disjoint from card LOADER-ABI's.
#[test]
fn the_refusal_vocabulary_is_closed_and_round_trips() {
    for refusal in ModeRefusal::ALL {
        assert_eq!(ModeRefusal::from_token(refusal.token()), Some(refusal));
        assert!(!refusal.as_str().is_empty());
        assert!(!refusal.reason().is_empty());
        assert_eq!(refusal.category(), ReasonCategory::StaleEvidence);
        assert_ne!(
            refusal.category().subject(),
            ReasonSubject::Environment,
            "{}: a contradicted mode is never an environment failure",
            refusal.as_str()
        );
    }
    assert_eq!(
        ModeRefusal::ALL.len(),
        ModeRefusal::ALL
            .iter()
            .map(|refusal| refusal.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        "every key is distinct"
    );
    let abi = qcore_c_gateway::legacy_c::AbiRefusal::ALL
        .iter()
        .map(|refusal| refusal.as_str())
        .collect::<BTreeSet<_>>();
    for refusal in ModeRefusal::ALL {
        assert!(
            !abi.contains(refusal.as_str()),
            "the two gates' keys must stay distinguishable in a report"
        );
    }
}

/// The two withdrawals compose in one unit, each keeping its own wording, so a
/// report can tell a signature disagreement from a custody-duty contradiction.
#[test]
fn both_withdrawals_compose_and_stay_distinguishable() {
    let contracts = load(&format!(
        "{AMBIENT}
extern fn m_sink(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
}};

extern fn m_note(n: usize) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
}};
"
    ));
    // `m_note` is declared `int` in C, which contradicts the declared `usize`
    // and is card LOADER-ABI's gate; `m_sink` is type-faithful and mode-
    // contradicted, which is this card's.
    let source = format!(
        "{AMBIENT_C}\
void m_note(int n);
void m_sink(void *p);
void m_sink(void *p) {{ m_free(p); }}
void caller(void) {{ void *h = m_alloc(8); m_note(1); m_sink(h); }}
"
    );
    let summary = analyse(&source, &contracts);
    let reported = unsupported(function(&summary, "caller"));
    assert!(
        reported.iter().any(|text| text.contains(
            "withdrawn against its C declaration in this translation unit \
             (abi-parameter-type-disagreement)"
        )),
        "{reported:?}"
    );
    assert!(
        reported
            .iter()
            .any(|text| text.contains("withdrawn against the summary derived from its body")),
        "{reported:?}"
    );
}

/// The same decision on the production entry point, and at certificate
/// issuance: a unit whose declaration this gate withdrew cannot be certified,
/// and the faithful control is.
#[test]
fn the_production_entry_point_and_the_certificate_agree_with_the_census() {
    if !clang_available() {
        eprintln!("skipping Clang-dependent LOADER-ABI-MODES test");
        return;
    }
    let contradicted = load(&format!(
        "{AMBIENT}
extern fn m_sink(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
}};
"
    ));
    let source = format!(
        "{AMBIENT_C}\
void m_sink(void *p);
void m_sink(void *p) {{ m_free(p); }}
void caller(void) {{ void *h = m_alloc(8); if (h) {{ m_sink(h); }} }}
"
    );
    match analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "caller",
        &contradicted,
    )
    .expect("Clang accepts the fixture")
    {
        // Measured: the withdrawal exposed the acquisition the declared
        // `Borrowed` was accounting for, so the production route reported a
        // **provisional** `Unsafe` (card LOADER-ABI's recorded
        // `LOADER-ABI-WITHDRAWAL-UNSAFE` direction). Card W44-LEAK-PIN-AUDIT:
        // the withdrawn call site is a refused statement handing `h` to
        // `m_sink`, so the refused hand-on rule withdraws that allegation and
        // the route answers `Unknown`; either way it never certifies.
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => {
            assert!(
                unsupported(function(&summary, "caller")).contains(&withdrawal(
                    "m_sink",
                    ModeRefusal::ParameterCustodyContradiction
                )),
                "the withdrawal is reported, so the allegation is never silent"
            );
        }
        other => panic!("a contradicted declaration must not certify: {other:?}"),
    }
    // The control: the same body under the callee's own faithful declaration,
    // which does certify, so the row above is not passing because nothing does.
    let faithful = load(&format!(
        "{AMBIENT}
extern fn m_sink(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_released(p, nonnull);
    ensures normal resource_return_none();
}};
"
    ));
    assert!(
        matches!(
            analyze_unrestricted_c11(
                &C11TranslationUnit {
                    source: &source,
                    ..Default::default()
                },
                "caller",
                &faithful,
            )
            .expect("Clang accepts the control"),
            UnrestrictedLegacyResult::Safe(_)
        ),
        "the agreeing control must still certify"
    );
}
