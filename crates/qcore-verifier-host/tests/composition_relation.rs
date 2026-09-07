//! E03 — the multi-provider composition relation, audited at the host.
//!
//! These tests build the host without the compiler, the audit driver or any
//! first-party package, and mint every composed certificate through the real
//! `verify_v2` firewall: nothing here fabricates a `CoreCertificate`.
//!
//! Scope. The subject is *composition*: several genuinely minted certificates
//! merged against a core-computed required set. The witness checker used to
//! mint them is a deliberately small test double — it is core-side code the
//! provider cannot reach, it decides its quantifiers against the core's own
//! obligation list, and it never compares provider data against provider data,
//! but it is not one of the production checkers. The production checkers'
//! own replay gates are E01/E02's subject and are not re-audited here.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use qcore_verifier_host::*;

const UNIT: &str = "unit.qc";
const PROFILE: &str = "qcore.fixture_composition.v1";

// ---------------------------------------------------------------------------
// Core side: a task builder and a small non-vacuous witness checker.
// ---------------------------------------------------------------------------

fn unit_identity(source: char) -> UnitIdentity {
    UnitIdentity {
        source_digest: source.to_string().repeat(64),
        build_digest: "b".repeat(64),
        translation_unit: UNIT.into(),
    }
}

fn target() -> TargetProfile {
    TargetProfile {
        library: "fixture".into(),
        revision: "source".into(),
        profile: "default".into(),
        dialect: "qcore".into(),
        platform: "generic".into(),
        backend: "native".into(),
    }
}

/// A core-derived task over `hooks`. Obligation identifiers are computed by
/// `VerificationTask::obligation_id`, never chosen here.
fn task(
    unit: UnitIdentity,
    hooks: &[&str],
    assumptions: BTreeMap<String, bool>,
    bounds: TaskBounds,
) -> VerificationTask {
    let mut task = VerificationTask {
        task_version: VERIFICATION_TASK_VERSION,
        unit,
        target: target(),
        semantic_profile: PROFILE.into(),
        contract_sources: BTreeMap::from([(UNIT.to_owned(), "c".repeat(64))]),
        bounds: bounds.clone(),
        assumptions: assumptions.clone(),
        evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        obligations: Vec::new(),
        derivation_incomplete: Vec::new(),
    };
    for hook in hooks {
        let subject = ObligationSubject::ObservedContract {
            function: (*hook).to_owned(),
            contract_module: UNIT.into(),
        };
        let obligation_id = task.obligation_id(
            VerifierCapability::ObservationHooks,
            ObligationProperty::ObservationChecked,
            &subject,
        );
        task.obligations.push(Obligation {
            obligation_id,
            capability: VerifierCapability::ObservationHooks,
            property: ObligationProperty::ObservationChecked,
            subject,
            scope: ObligationScope::ObservedRuns {
                run_ids: BTreeSet::new(),
            },
            bounds: bounds.clone(),
            assumptions: assumptions.clone(),
            evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        });
    }
    task.obligations.sort();
    task.validate().expect("fixture task is well formed");
    task
}

fn simple_task(unit: char, hooks: &[&str]) -> VerificationTask {
    task(
        unit_identity(unit),
        hooks,
        BTreeMap::from([("observations_trusted".to_owned(), true)]),
        TaskBounds::observed_runs(),
    )
}

/// Test-double replay. The denominator is the core's own obligation list; a
/// hook the witness does not record is simply not discharged, and the firewall
/// compares the discharged set to the core's required set.
struct HookReplay;

impl WitnessChecker for HookReplay {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ObservationHooks
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        let SemanticWitness::ContractObservation(witness) = witness else {
            return Err("fixture checker replays contract observations only".into());
        };
        if witness.runs.is_empty() {
            return Err("no observed run".into());
        }
        let mut discharged = BTreeSet::new();
        for obligation in task.obligations_for(VerifierCapability::ObservationHooks) {
            let ObligationSubject::ObservedContract { function, .. } = &obligation.subject else {
                continue;
            };
            let observed = witness.runs.iter().any(|run| {
                run.observations
                    .iter()
                    .any(|record| &record.hook == function && record.values.get("ok") == Some(&1))
            });
            if observed {
                discharged.insert(obligation.obligation_id.clone());
            }
        }
        Ok(discharged)
    }
}

// ---------------------------------------------------------------------------
// Provider side.
// ---------------------------------------------------------------------------

fn manifest(id: &str) -> PluginManifest {
    PluginManifest {
        plugin_id: PluginId::new(id).unwrap(),
        package_version: "0.1.0".into(),
        protocol_versions: BTreeSet::from([PLUGIN_PROTOCOL_VERSION]),
        evidence_versions: BTreeSet::from([PLUGIN_EVIDENCE_VERSION, PLUGIN_EVIDENCE_VERSION_V2]),
        entrypoint: PluginEntrypoint::BuiltIn,
        package_digest: format!("builtin:{id}"),
        targets: vec![SupportedTarget {
            library: "fixture".into(),
            revisions: BTreeSet::from(["source".into()]),
            profiles: BTreeSet::from(["default".into()]),
            dialects: BTreeSet::from(["qcore".into()]),
            platforms: BTreeSet::from(["generic".into()]),
            backends: BTreeSet::from(["native".into()]),
        }],
        capabilities: BTreeSet::from([VerifierCapability::ObservationHooks]),
        required_foreign_summaries: BTreeSet::new(),
        required_observation_hooks: BTreeSet::new(),
        filesystem_inputs: BTreeSet::new(),
        network_access: false,
        limits: PluginLimits::default(),
        trust_class: PluginTrustClass::FirstParty,
        certificate_checker: "fixture.hook_replay.v2".into(),
        lean_theorem_family: "QCore.VerifierPlugin".into(),
    }
}

/// What the fixture provider answers. Every variant reaches the same firewall.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Answer {
    Honest,
    /// A run with no matching observation: an honest envelope, unchecked paths.
    Silent,
    Unknown,
    Alleged,
    Fail,
}

struct Fixture {
    manifest: PluginManifest,
    answer: Answer,
}

impl VerifierProvider for Fixture {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn verify(&self, _: &VerificationRequest) -> Result<VerificationResponse, ProviderFailure> {
        Err(ProviderFailure::Protocol(
            "fixture speaks evidence v2".into(),
        ))
    }

    fn verify_v2(
        &self,
        request: &VerificationRequestV2,
    ) -> Result<VerificationResponseV2, ProviderFailure> {
        if self.answer == Answer::Fail {
            return Err(ProviderFailure::Protocol("fixture provider failed".into()));
        }
        let envelope = |verdict, evidence| VerificationResponseV2 {
            protocol_version: request.protocol_version,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict,
            diagnostics: vec![PluginDiagnostic {
                code: format!("{}.N01", request.plugin_id),
                message: "fixture note".into(),
                fact_id: None,
            }],
            evidence,
        };
        match self.answer {
            Answer::Unknown => return Ok(envelope(ProposedVerdict::Unknown, None)),
            Answer::Alleged => return Ok(envelope(ProposedVerdict::Unsafe, None)),
            _ => {}
        }
        let observations = if self.answer == Answer::Silent {
            vec![ContractObservationRecord {
                function: "unrelated".into(),
                hook: "unrelated".into(),
                values: BTreeMap::from([("ok".into(), 1)]),
            }]
        } else {
            request
                .obligations
                .iter()
                .filter_map(|obligation| match &obligation.subject {
                    ObligationSubject::ObservedContract { function, .. } => {
                        Some(ContractObservationRecord {
                            function: function.clone(),
                            hook: function.clone(),
                            values: BTreeMap::from([("ok".into(), 1)]),
                        })
                    }
                    _ => None,
                })
                .collect()
        };
        Ok(envelope(
            ProposedVerdict::Safe,
            Some(ProposedEvidenceV2 {
                evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
                unit: request.unit.clone(),
                capability: request.capability,
                coverage: request
                    .obligations
                    .iter()
                    .map(|obligation| obligation.obligation_id.clone())
                    .collect(),
                assumptions_used: request.assumptions.clone(),
                reproducibility: Reproducibility {
                    provider_version: self.manifest.package_version.clone(),
                    package_digest: self.manifest.package_digest.clone(),
                    deterministic: true,
                },
                witness: SemanticWitness::ContractObservation(ContractObservationWitness {
                    task_digest: request.task_digest.clone(),
                    runs: vec![ObservedRun {
                        run_id: "fixture-run".into(),
                        binding: NativeRunBinding {
                            emitted_source_digest: "a".repeat(64),
                            executable_digest: "b".repeat(64),
                            compiler_invocation: "cc fixture.c".into(),
                            exit_status: 0,
                            seed_or_input: "fixture".into(),
                            profile: "default".into(),
                            instrumentation_version: "fixture-v1".into(),
                        },
                        observations,
                    }],
                }),
            }),
        ))
    }
}

fn registry(providers: &[(&str, Answer)]) -> VerifierRegistry {
    let mut registry = VerifierRegistry::default();
    for (id, answer) in providers {
        registry
            .register(Arc::new(Fixture {
                manifest: manifest(id),
                answer: *answer,
            }))
            .expect("fixture manifests are valid");
    }
    registry
}

fn request(plugin: &str, task: &VerificationTask) -> VerificationRequestV2 {
    VerificationRequestV2 {
        source_effect: None,
        counter_effect: None,
        reactor_program: None,
        protocol_version: PLUGIN_PROTOCOL_VERSION,
        evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        request_id: format!("{plugin}:{}", &task.task_digest()[..16]),
        plugin_id: PluginId::new(plugin).unwrap(),
        capability: VerifierCapability::ObservationHooks,
        target: task.target.clone(),
        unit: task.unit.clone(),
        assumptions: task.assumptions.clone(),
        task_digest: task.task_digest(),
        obligations: task.obligations.clone(),
        facts: task
            .obligations
            .iter()
            .map(|obligation| NormalizedFact {
                fact_id: obligation.obligation_id.clone(),
                subject: obligation.subject.canonical(),
                predicate: FactPredicate::ObservationMatched,
                holds: true,
            })
            .collect(),
    }
}

fn mint(registry: &VerifierRegistry, plugin: &str, task: &VerificationTask) -> VerificationOutcome {
    registry.verify_v2(&request(plugin, task), task, Some(&HookReplay))
}

fn codes(outcome: &VerificationOutcome) -> Vec<&str> {
    outcome
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

fn union(tasks: &[&VerificationTask]) -> BTreeSet<String> {
    tasks
        .iter()
        .flat_map(|task| task.coverage_for(VerifierCapability::ObservationHooks))
        .collect()
}

// ---------------------------------------------------------------------------
// Acceptance 1 — the requested nonempty obligations must all be covered.
// ---------------------------------------------------------------------------

#[test]
fn disjoint_providers_merge_over_the_exact_core_required_set() {
    let registry = registry(&[
        ("example.left", Answer::Honest),
        ("example.right", Answer::Honest),
    ]);
    let left = simple_task('1', &["open", "close"]);
    let right = simple_task('1', &["flush"]);
    let outcomes = vec![
        mint(&registry, "example.left", &left),
        mint(&registry, "example.right", &right),
    ];
    assert!(
        outcomes
            .iter()
            .all(|outcome| outcome.verdict == ProposedVerdict::Safe)
    );
    let required = union(&[&left, &right]);
    assert_eq!(required.len(), 3, "three distinct core-derived obligations");

    let composed = compose_verification_with_inventory(outcomes.clone(), &required);
    assert_eq!(
        composed.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        composed.diagnostics
    );
    // The composite is the certificate collection, not the representative.
    let composite = compose_certificates(&outcomes).expect("safe composition");
    assert_eq!(composite.certificates().len(), 2);
    let covered = composite
        .certificates()
        .iter()
        .flat_map(|certificate| certificate.coverage().iter().cloned())
        .collect::<BTreeSet<_>>();
    assert_eq!(covered, required);
}

#[test]
fn an_unknown_or_failed_provider_leaves_its_obligations_uncovered() {
    for answer in [Answer::Unknown, Answer::Fail, Answer::Silent] {
        let registry = registry(&[("example.left", Answer::Honest), ("example.right", answer)]);
        let left = simple_task('1', &["open", "close"]);
        let right = simple_task('1', &["flush"]);
        let good = mint(&registry, "example.left", &left);
        let bad = mint(&registry, "example.right", &right);
        assert_eq!(good.verdict, ProposedVerdict::Safe);
        assert_eq!(bad.verdict, ProposedVerdict::Unknown);
        assert!(bad.certificate.is_none());
        // A silent provider is refused by the firewall, not by composition:
        // its witness discharges nothing, so QP009 fires before any merge.
        if answer == Answer::Silent {
            assert!(codes(&bad).contains(&"QP009"));
        }
        let required = union(&[&left, &right]);
        let composed = compose_verification_with_inventory(vec![good.clone(), bad], &required);
        assert_eq!(composed.verdict, ProposedVerdict::Unknown);
        assert_eq!(codes(&composed), vec!["QP011"]);
        // The uncovered obligations are still in the core's required set.
        let covered = good.certificate.as_ref().unwrap().coverage().clone();
        assert!(!required.difference(&covered).collect::<Vec<_>>().is_empty());
    }
}

#[test]
fn a_provider_that_never_answered_is_not_an_empty_requirement() {
    // The provider is simply absent from the outcome list. Composition must
    // not read "nothing was reported" as "nothing was required".
    let registry = registry(&[("example.left", Answer::Honest)]);
    let left = simple_task('1', &["open"]);
    let right = simple_task('1', &["flush"]);
    let composed = compose_verification_with_inventory(
        vec![mint(&registry, "example.left", &left)],
        &union(&[&left, &right]),
    );
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP011"]);
    assert!(composed.certificate.is_none());
}

#[test]
fn an_empty_required_set_is_never_composed_safe() {
    let registry = registry(&[("example.left", Answer::Honest)]);
    let left = simple_task('1', &["open"]);
    let composed = compose_verification_with_inventory(
        vec![mint(&registry, "example.left", &left)],
        &BTreeSet::new(),
    );
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP016"]);
    assert_eq!(
        compose_verification_with_inventory(vec![], &union(&[&left])).verdict,
        ProposedVerdict::Unknown
    );
    assert_eq!(codes(&compose_verification(vec![])), vec!["QP010"]);
}

#[test]
fn coverage_beyond_the_required_set_is_refused() {
    let registry = registry(&[("example.left", Answer::Honest)]);
    let wide = simple_task('1', &["open", "close"]);
    let narrow = simple_task('1', &["open"]);
    let composed = compose_verification_with_inventory(
        vec![mint(&registry, "example.left", &wide)],
        &union(&[&narrow]),
    );
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP011"]);
}

// ---------------------------------------------------------------------------
// Acceptance 2 — incompatible identities, premises and bounds; duplicates.
// ---------------------------------------------------------------------------

#[test]
fn duplicate_evidence_is_not_counted_twice() {
    let registry = registry(&[("example.left", Answer::Honest)]);
    let left = simple_task('1', &["open"]);
    let right = simple_task('1', &["flush"]);
    let one = mint(&registry, "example.left", &left);
    let required = union(&[&left, &right]);
    assert_eq!(required.len(), 2);
    // The same certificate submitted twice covers one obligation, not two.
    let composed = compose_verification_with_inventory(vec![one.clone(), one.clone()], &required);
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP011"]);
    assert!(composed.diagnostics[0].message.contains('1'));
    // Duplicating it against its own required set stays Safe: duplication
    // neither adds nor removes coverage.
    assert_eq!(
        compose_verification_with_inventory(vec![one.clone(), one], &union(&[&left])).verdict,
        ProposedVerdict::Safe
    );
}

#[test]
fn certificates_over_different_units_do_not_merge() {
    let registry = registry(&[
        ("example.left", Answer::Honest),
        ("example.right", Answer::Honest),
    ]);
    let left = simple_task('1', &["open"]);
    let right = simple_task('2', &["flush"]);
    assert_ne!(left.unit, right.unit);
    let composed = compose_verification(vec![
        mint(&registry, "example.left", &left),
        mint(&registry, "example.right", &right),
    ]);
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP015"]);
}

#[test]
fn conflicting_premises_do_not_merge() {
    let registry = registry(&[
        ("example.left", Answer::Honest),
        ("example.right", Answer::Honest),
    ]);
    let trusted = task(
        unit_identity('1'),
        &["open"],
        BTreeMap::from([("observations_trusted".to_owned(), true)]),
        TaskBounds::observed_runs(),
    );
    let distrusted = task(
        unit_identity('1'),
        &["flush"],
        BTreeMap::from([("observations_trusted".to_owned(), false)]),
        TaskBounds::observed_runs(),
    );
    let composed = compose_verification(vec![
        mint(&registry, "example.left", &trusted),
        mint(&registry, "example.right", &distrusted),
    ]);
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP012"]);
    assert!(
        composed.diagnostics[0]
            .message
            .contains("conflicting assumptions")
    );
}

#[test]
fn a_composite_may_not_understate_its_premise_set() {
    // Non-conflicting but unequal premise sets. The published representative
    // certificate carries one provider's assumptions, so merging these would
    // drop `callbacks_bounded` from the published result.
    let registry = registry(&[
        ("example.left", Answer::Honest),
        ("example.right", Answer::Honest),
    ]);
    let narrow = task(
        unit_identity('1'),
        &["open"],
        BTreeMap::from([("observations_trusted".to_owned(), true)]),
        TaskBounds::observed_runs(),
    );
    let wide = task(
        unit_identity('1'),
        &["flush"],
        BTreeMap::from([
            ("observations_trusted".to_owned(), true),
            ("callbacks_bounded".to_owned(), false),
        ]),
        TaskBounds::observed_runs(),
    );
    for order in [[&narrow, &wide], [&wide, &narrow]] {
        let composed = compose_verification(vec![
            mint(&registry, "example.left", order[0]),
            mint(&registry, "example.right", order[1]),
        ]);
        assert_eq!(composed.verdict, ProposedVerdict::Unknown);
        assert_eq!(codes(&composed), vec!["QP012"]);
        assert!(composed.diagnostics[0].message.contains("understate"));
    }
    // Control: the identical premise set composes, so the refusal above is
    // attributable to the premise set alone.
    let matched = task(
        unit_identity('1'),
        &["flush"],
        narrow.assumptions.clone(),
        TaskBounds::observed_runs(),
    );
    let composed = compose_verification(vec![
        mint(&registry, "example.left", &narrow),
        mint(&registry, "example.right", &matched),
    ]);
    assert_eq!(
        composed.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        composed.diagnostics
    );
    assert_eq!(
        composed
            .certificate
            .as_ref()
            .unwrap()
            .evidence()
            .assumptions_used,
        narrow.assumptions
    );
}

#[test]
fn one_obligation_discharged_under_two_bounds_does_not_merge() {
    let registry = registry(&[
        ("example.left", Answer::Honest),
        ("example.right", Answer::Honest),
    ]);
    let observed = simple_task('1', &["open"]);
    let single = task(
        unit_identity('1'),
        &["open"],
        observed.assumptions.clone(),
        TaskBounds::single_path(),
    );
    // Identical unit, profile and subject, so the core computes the same
    // obligation identifier under two different bounds.
    assert_eq!(
        observed.coverage_for(VerifierCapability::ObservationHooks),
        single.coverage_for(VerifierCapability::ObservationHooks)
    );
    assert_ne!(observed.task_digest(), single.task_digest());
    let composed = compose_verification_with_inventory(
        vec![
            mint(&registry, "example.left", &observed),
            mint(&registry, "example.right", &single),
        ],
        &union(&[&observed]),
    );
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP013"]);
    assert!(
        composed.diagnostics[0]
            .message
            .contains("incomparable bounds")
    );
}

// ---------------------------------------------------------------------------
// Acceptance 3 — a violation stays scoped and unpromoted.
// ---------------------------------------------------------------------------

#[test]
fn a_provider_allegation_is_never_composed_into_a_checked_violation() {
    let registry = registry(&[
        ("example.left", Answer::Honest),
        ("example.right", Answer::Alleged),
    ]);
    let left = simple_task('1', &["open"]);
    let right = simple_task('1', &["flush"]);
    let alleged = mint(&registry, "example.right", &right);
    // The firewall already downgrades: no route through the registry returns
    // `Unsafe`, and the provider's own diagnostic is retained.
    assert_eq!(alleged.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&alleged), vec!["QP010", "example.right.N01"]);
    assert!(alleged.certificate.is_none());

    let composed = compose_verification(vec![mint(&registry, "example.left", &left), alleged]);
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP011"]);
}

#[test]
fn composition_cannot_republish_an_unchecked_unsafe_label() {
    // A caller-supplied `Unsafe` — the shape a source-unit provisional
    // diagnostic would have — must not become the composite verdict. No
    // evidence version ships a counterexample checker.
    let registry = registry(&[("example.left", Answer::Honest)]);
    let left = simple_task('1', &["open"]);
    let alleged = VerificationOutcome {
        verdict: ProposedVerdict::Unsafe,
        diagnostics: vec![PluginDiagnostic {
            code: "example.left.N02".into(),
            message: "provisional resource-unit finding".into(),
            fact_id: None,
        }],
        certificate: None,
    };
    for outcomes in [
        vec![alleged.clone()],
        vec![mint(&registry, "example.left", &left), alleged.clone()],
        vec![alleged.clone(), mint(&registry, "example.left", &left)],
    ] {
        let composed = compose_verification(outcomes.clone());
        assert_eq!(composed.verdict, ProposedVerdict::Unknown);
        assert_eq!(codes(&composed), vec!["QP010", "example.left.N02"]);
        assert!(composed.certificate.is_none());
        assert_eq!(
            compose_verification_with_inventory(outcomes.clone(), &union(&[&left])).verdict,
            ProposedVerdict::Unknown
        );
        assert!(compose_certificates(&outcomes).is_err());
    }
}

#[test]
fn a_safe_verdict_without_a_certificate_never_composes() {
    let forged = VerificationOutcome {
        verdict: ProposedVerdict::Safe,
        diagnostics: vec![],
        certificate: None,
    };
    let composed = compose_verification(vec![forged]);
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP014"]);
}
