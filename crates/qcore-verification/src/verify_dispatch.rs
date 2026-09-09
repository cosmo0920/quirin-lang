//! W3 verify-scope dispatch (closes VP05).
//!
//! Implements `docs/design/W3_OBLIGATIONS_EVIDENCE_V2.md` §3. Before W3 a
//! resolved `verify scope` was rejected as configuration-only; it now drives
//! real verification:
//!
//! ```text
//! resolved scope (resolve_source_scopes — unchanged, still fail-closed)
//!   → per (provider use, capability):
//!       core derives the obligation inventory / VerificationTask
//!       → empty-or-incomplete derivation handling (§3.5)
//!       → request (protocol 1; evidence 2 when the provider and a core
//!         checker support it, else evidence 1 whose Safe stays refused)
//!       → registry gates QP001–QP005 → provider → firewall QP006–QP016
//!   → per scope: composition over all outcomes, plus the required-set check
//! ```
//!
//! There is no retry, no partial promotion, and no optimistic path: a scope is
//! verified only when every obligation of every requested capability is
//! discharged by a core-checked certificate.

use std::collections::{BTreeMap, BTreeSet};

use qcore_frontend::frontend::SourceProgram;
use qcore_schemas::obligation::ObligationProperty;

use crate::obligation_derivation::{CoreReplayInputs, DerivationInputs, DerivedTask, derive_task};
use crate::verifier_plugin::{
    FactPredicate, NormalizedFact, PLUGIN_EVIDENCE_VERSION, PLUGIN_EVIDENCE_VERSION_V2,
    PLUGIN_PROTOCOL_VERSION, PluginDiagnostic, PluginManifest, ProposedVerdict,
    ResolvedVerifierScope, ResolvedVerifierUse, TargetProfile, VerificationOutcome,
    VerificationRequest, VerificationRequestV2, VerificationTask, VerifierCapability,
    VerifierRegistry, WitnessChecker, compose_verification_with_inventory,
};
use crate::witness_replay::{ChunkLifecycleReplay, ContractObservationReplay};

/// A language-level diagnostic produced by dispatching one scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeDiagnostic {
    /// `Q701` (obligations undischarged), `Q702` (conflicting evidence), or
    /// `Q703` (unverified provider allegation).
    pub code: &'static str,
    pub scope: String,
    pub message: String,
}

/// The result of dispatching one `(provider, capability)` pair.
#[derive(Clone, Debug)]
pub struct CapabilityDispatch {
    pub plugin_id: String,
    pub capability: VerifierCapability,
    pub task: VerificationTask,
    pub outcome: VerificationOutcome,
    /// `true` when a provider process/adapter was actually invoked.
    pub dispatched: bool,
}

#[derive(Clone, Debug)]
pub struct ScopeDispatch {
    pub scope: String,
    pub capabilities: Vec<CapabilityDispatch>,
    pub composed: VerificationOutcome,
    pub diagnostics: Vec<ScopeDiagnostic>,
}

impl ScopeDispatch {
    /// A scope is verified only when composition returned `Safe` over the full
    /// core-derived obligation set.
    #[must_use]
    pub fn verified(&self) -> bool {
        self.composed.verdict == ProposedVerdict::Safe && self.diagnostics.is_empty()
    }
}

#[derive(Clone, Debug, Default)]
pub struct DispatchReport {
    pub scopes: Vec<ScopeDispatch>,
}

impl DispatchReport {
    #[must_use]
    pub fn diagnostics(&self) -> Vec<ScopeDiagnostic> {
        self.scopes
            .iter()
            .flat_map(|scope| scope.diagnostics.iter().cloned())
            .collect()
    }
}

fn predicate_for(property: ObligationProperty) -> FactPredicate {
    match property {
        ObligationProperty::BooleanFieldContract
        | ObligationProperty::CounterUpdateContract
        | ObligationProperty::ListUnlinkContract
        | ObligationProperty::FieldDestructorContract
        | ObligationProperty::CallResultUseContract
        | ObligationProperty::CustodyDischarged
        | ObligationProperty::ParameterModeRespected
        | ObligationProperty::NoUnknownCallEffect => FactPredicate::ResourceBalanced,
        ObligationProperty::ContextDischarged
        | ObligationProperty::StreamDischarged
        | ObligationProperty::ChunkDischarged
        | ObligationProperty::TransactionSettled
        | ObligationProperty::LifecycleProgramComplete => FactPredicate::LifecycleTerminal,
        ObligationProperty::ObservationChecked
        | ObligationProperty::ObservationCoverageComplete => FactPredicate::ObservationMatched,
        ObligationProperty::ProjectionAccounted => FactPredicate::ProjectionComplete,
        ObligationProperty::EventGenerationDischarged
        | ObligationProperty::CallbackQuiescentAtTeardown => FactPredicate::ApiRecognized,
        ObligationProperty::MutexGuardReleased | ObligationProperty::JoinBeforeExit => {
            FactPredicate::ConcurrencySafe
        }
    }
}

/// Bounded source facts derived from the core inventory. In v2 they are
/// envelope compatibility only: the obligations, not these, are the carrier.
fn facts(task: &VerificationTask, capability: VerifierCapability) -> Vec<NormalizedFact> {
    task.obligations_for(capability)
        .into_iter()
        .map(|obligation| NormalizedFact {
            fact_id: obligation.obligation_id.clone(),
            subject: obligation.subject.canonical(),
            predicate: predicate_for(obligation.property),
            holds: true,
        })
        .collect()
}

/// The target profile this unit presents to a provider. A manifest that
/// declares no matching target simply fails QP005 later; nothing is guessed.
fn target_profile(manifest: &PluginManifest, selected: &ResolvedVerifierUse) -> TargetProfile {
    let candidate = |library: &str| TargetProfile {
        library: library.to_owned(),
        revision: "source".into(),
        profile: selected.profile.clone(),
        dialect: "qcore".into(),
        platform: std::env::consts::OS.to_owned(),
        backend: "native".into(),
    };
    for target in &manifest.targets {
        let profile = candidate(&target.library);
        if manifest
            .targets
            .iter()
            .any(|declared| declared.library == profile.library)
            && selected
                .capabilities
                .iter()
                .all(|capability| manifest.supports(*capability, &profile))
        {
            return profile;
        }
    }
    candidate(
        manifest
            .targets
            .first()
            .map_or("", |target| target.library.as_str()),
    )
}

fn checker_for<'a>(
    capability: VerifierCapability,
    replay: &'a CoreReplayInputs,
) -> Option<Box<dyn WitnessChecker + 'a>> {
    match capability {
        VerifierCapability::ChunkLifecycle => replay.lifecycle.as_ref().map(|inventory| {
            Box::new(ChunkLifecycleReplay::new(inventory)) as Box<dyn WitnessChecker>
        }),
        VerifierCapability::ObservationHooks => replay.observation.as_ref().map(|inventory| {
            Box::new(ContractObservationReplay::new(inventory)) as Box<dyn WitnessChecker>
        }),
        // No trusted checker exists for these capabilities at evidence v2.0:
        // a v2 `Safe` proposal hits QP016 and stays `Unknown` (design §2.5).
        VerifierCapability::ResourceContracts
        | VerifierCapability::EventTraces
        | VerifierCapability::Concurrency
        | VerifierCapability::TranslationUnitProjection => None,
    }
}

fn unknown(code: &str, message: impl Into<String>) -> VerificationOutcome {
    VerificationOutcome {
        verdict: ProposedVerdict::Unknown,
        diagnostics: vec![PluginDiagnostic {
            code: code.to_owned(),
            message: message.into(),
            fact_id: None,
        }],
        certificate: None,
        composite: None,
    }
}

/// Dispatches every resolved verify scope of a compilation unit.
#[must_use]
pub fn dispatch_scopes(
    program: &SourceProgram,
    translation_unit: &str,
    scopes: &[ResolvedVerifierScope],
    registry: &VerifierRegistry,
) -> DispatchReport {
    let mut report = DispatchReport::default();
    for scope in scopes {
        report
            .scopes
            .push(dispatch_scope(program, translation_unit, scope, registry));
    }
    report
}

fn dispatch_scope(
    program: &SourceProgram,
    translation_unit: &str,
    scope: &ResolvedVerifierScope,
    registry: &VerifierRegistry,
) -> ScopeDispatch {
    let mut capabilities = Vec::new();
    let mut required = BTreeSet::new();
    let mut outcomes = Vec::new();
    let mut diagnostics = Vec::new();
    let mut absent = Vec::new();
    // A scope that resolved to no provider has nothing to discharge its
    // obligations and is unverified, never vacuously safe.
    if scope.providers.is_empty() {
        diagnostics.push(ScopeDiagnostic {
            code: "Q701",
            scope: scope.name.clone(),
            message: "verify scope resolved to no provider; its obligations are undischarged"
                .into(),
        });
    }
    for selected in &scope.providers {
        let Some(manifest) = registry.manifest(&selected.plugin_id).cloned() else {
            // E03-B4. Without the manifest there is no target profile and no
            // required-hook denominator, so this provider's obligations cannot
            // be derived and never enter `required` — which used to leave
            // `composed.verdict` free to read `Safe` off the providers that
            // did answer, with only `verified()` and `Q700` standing between
            // that and a caller. Absence is now a refusal in the composed
            // verdict itself (`QP002`, the registry's own code for a plugin
            // that is not installed), so no consumer reading `composed` alone
            // can see `Safe` for a scope one of whose providers is missing.
            absent.push(selected.plugin_id.as_str().to_owned());
            continue;
        };
        let target = target_profile(&manifest, selected);
        for &capability in &selected.capabilities {
            let inputs = DerivationInputs {
                translation_unit,
                program,
                target: target.clone(),
                contract_sources: BTreeMap::from([(
                    translation_unit.to_owned(),
                    program.source().to_owned(),
                )]),
                required_observation_hooks: manifest.required_observation_hooks.clone(),
                assumptions: BTreeMap::new(),
                // E03-B3: provider-indexed obligations are keyed by this.
                verifier: Some(selected.plugin_id.as_str().to_owned()),
            };
            let DerivedTask { task, replay } = derive_task(&inputs, capability);
            let coverage = task.coverage_for(capability);
            let (outcome, dispatched) =
                dispatch_capability(registry, &manifest, selected, capability, &task, &replay);
            for id in &coverage {
                required.insert(id.clone());
            }
            if outcome
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "QP010")
            {
                diagnostics.push(ScopeDiagnostic {
                    code: "Q703",
                    scope: scope.name.clone(),
                    message: format!(
                        "plugin `{}` alleges a violation of capability `{capability}` that the \
                         core cannot validate; evidence v2 has no counterexample checker",
                        selected.plugin_id
                    ),
                });
            }
            outcomes.push(outcome.clone());
            capabilities.push(CapabilityDispatch {
                plugin_id: selected.plugin_id.as_str().to_owned(),
                capability,
                task,
                outcome,
                dispatched,
            });
        }
    }
    let composed = if !absent.is_empty() {
        let missing = absent.join("`, `");
        // Absence dominates: an inventory that is missing a provider's
        // obligations entirely is not a smaller inventory, it is an unknown
        // one. Reported before the empty-inventory and merge branches so the
        // composed diagnostic names the cause.
        unknown(
            "QP002",
            format!(
                "plugin `{missing}` is not installed; its obligations could be neither derived \
                 nor discharged, so this scope has no composite"
            ),
        )
    } else if required.is_empty() {
        // §3.5, first increment: a derivation with zero obligations is not
        // promoted. This costs completeness, never soundness.
        unknown(
            "QP016",
            "no obligations derived; empty-scope promotion requires a derivation-coverage \
             witness",
        )
    } else {
        compose_verification_with_inventory(outcomes, &required)
    };
    if composed.verdict != ProposedVerdict::Safe {
        let conflicting = composed
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.code.as_str(), "QP012" | "QP013"));
        let detail = composed
            .diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .chain(capabilities.iter().flat_map(|dispatch| {
                dispatch
                    .outcome
                    .diagnostics
                    .iter()
                    .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            }))
            .collect::<Vec<_>>()
            .join("; ");
        diagnostics.push(if conflicting {
            ScopeDiagnostic {
                code: "Q702",
                scope: scope.name.clone(),
                message: format!(
                    "verify scope is unverified: providers returned conflicting evidence or \
                     assumptions [{detail}]"
                ),
            }
        } else {
            ScopeDiagnostic {
                code: "Q701",
                scope: scope.name.clone(),
                message: format!(
                    "verify scope is unverified: one or more core-derived obligations were \
                     not discharged by core-checked evidence [{detail}]"
                ),
            }
        });
    }
    ScopeDispatch {
        scope: scope.name.clone(),
        capabilities,
        composed,
        diagnostics,
    }
}

fn dispatch_capability(
    registry: &VerifierRegistry,
    manifest: &PluginManifest,
    selected: &ResolvedVerifierUse,
    capability: VerifierCapability,
    task: &VerificationTask,
    replay: &CoreReplayInputs,
) -> (VerificationOutcome, bool) {
    if task.coverage_for(capability).is_empty() {
        // Nothing to discharge means nothing for a provider to do; no request
        // is sent (design §3.5).
        return (
            unknown(
                "QP016",
                format!(
                    "no obligations derived for `{capability}`; empty-scope promotion requires \
                     a derivation-coverage witness"
                ),
            ),
            false,
        );
    }
    let request_id = format!(
        "{}:{}:{}",
        selected.plugin_id,
        capability,
        &task.task_digest()[..16]
    );
    let target = task.target.clone();
    let checker = checker_for(capability, replay);
    let supports_v2 = manifest
        .evidence_versions
        .contains(&PLUGIN_EVIDENCE_VERSION_V2);
    if supports_v2 {
        let request = VerificationRequestV2 {
            source_effect: None,
            counter_effect: None,
            reactor_program: None,
            teardown_source: None,
            call_use_source: None,
            protocol_version: PLUGIN_PROTOCOL_VERSION,
            evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
            request_id,
            plugin_id: selected.plugin_id.clone(),
            capability,
            target,
            unit: task.unit.clone(),
            facts: facts(task, capability),
            assumptions: task.assumptions.clone(),
            task_digest: task.task_digest(),
            obligations: task
                .obligations_for(capability)
                .into_iter()
                .cloned()
                .collect(),
        };
        let outcome = registry.verify_v2(&request, task, checker.as_deref());
        let dispatched = !task.is_incomplete_for(capability)
            && !outcome.diagnostics.iter().any(|diagnostic| {
                matches!(
                    diagnostic.code.as_str(),
                    "QP001" | "QP002" | "QP003" | "QP004" | "QP005"
                )
            });
        return (
            enforce_derivation_completeness(task, capability, outcome),
            dispatched,
        );
    }
    // The provider does not speak evidence v2. Dispatch still happens under
    // v1, where the terminal v1 refusal keeps every `Safe` proposal out of a
    // certificate; the scope stays unverified.
    let request = VerificationRequest {
        protocol_version: PLUGIN_PROTOCOL_VERSION,
        evidence_version: PLUGIN_EVIDENCE_VERSION,
        request_id,
        plugin_id: selected.plugin_id.clone(),
        capability,
        target,
        unit: task.unit.clone(),
        facts: facts(task, capability),
        assumptions: task.assumptions.clone(),
    };
    let outcome = registry.verify(&request);
    let dispatched = !outcome.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.code.as_str(),
            "QP001" | "QP002" | "QP003" | "QP004" | "QP005"
        )
    });
    (
        enforce_derivation_completeness(task, capability, outcome),
        dispatched,
    )
}

/// A capability whose derivation could not classify every construct stays
/// `Unknown` whatever the provider returned (design §1.4 / §3.5). Dispatch
/// still happens — the provider's own diagnostics are surfaced — but no
/// response can promote an inventory the core knows is incomplete.
fn enforce_derivation_completeness(
    task: &VerificationTask,
    capability: VerifierCapability,
    outcome: VerificationOutcome,
) -> VerificationOutcome {
    if !task.is_incomplete_for(capability) {
        return outcome;
    }
    let reasons = task
        .derivation_incomplete
        .iter()
        .filter(|gap| gap.capability == capability)
        .map(|gap| gap.reason.clone())
        .collect::<Vec<_>>()
        .join("; ");
    let mut forced = unknown(
        "QP016",
        format!("the core derivation is incomplete for `{capability}`: {reasons}"),
    );
    forced.diagnostics.extend(outcome.diagnostics);
    forced
}
