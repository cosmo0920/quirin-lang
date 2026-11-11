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

/// A second capability over the same unit identity, so a multi-capability
/// composition can be exercised at the merge (E03-B2).
fn resource_task(unit: char, functions: &[&str]) -> VerificationTask {
    let assumptions = BTreeMap::from([("observations_trusted".to_owned(), true)]);
    let bounds = TaskBounds::observed_runs();
    let mut task = VerificationTask {
        task_version: VERIFICATION_TASK_VERSION,
        unit: unit_identity(unit),
        target: target(),
        semantic_profile: "qcore.fixture_composition_resources.v1".into(),
        contract_sources: BTreeMap::from([(UNIT.to_owned(), "c".repeat(64))]),
        bounds: bounds.clone(),
        assumptions: assumptions.clone(),
        evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        obligations: Vec::new(),
        derivation_incomplete: Vec::new(),
    };
    for symbol in functions {
        let subject = ObligationSubject::Function {
            symbol: (*symbol).to_owned(),
            translation_unit: UNIT.into(),
        };
        let obligation_id = task.obligation_id(
            VerifierCapability::ResourceContracts,
            ObligationProperty::NoUnknownCallEffect,
            &subject,
        );
        task.obligations.push(Obligation {
            obligation_id,
            capability: VerifierCapability::ResourceContracts,
            property: ObligationProperty::NoUnknownCallEffect,
            subject,
            scope: ObligationScope::Functions {
                symbols: BTreeSet::from([(*symbol).to_owned()]),
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

/// The same test double for the second capability, keyed by function symbol.
struct ResourceReplay;

impl WitnessChecker for ResourceReplay {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
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
        for obligation in task.obligations_for(VerifierCapability::ResourceContracts) {
            let ObligationSubject::Function { symbol, .. } = &obligation.subject else {
                continue;
            };
            let observed = witness.runs.iter().any(|run| {
                run.observations
                    .iter()
                    .any(|record| &record.hook == symbol && record.values.get("ok") == Some(&1))
            });
            if observed {
                discharged.insert(obligation.obligation_id.clone());
            }
        }
        Ok(discharged)
    }
}

/// The name a fixture witness records for one obligation, whichever capability
/// derived it.
fn observed_name(subject: &ObligationSubject) -> Option<&str> {
    match subject {
        ObligationSubject::ObservedContract { function, .. } => Some(function),
        ObligationSubject::Function { symbol, .. } => Some(symbol),
        _ => None,
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
        capabilities: BTreeSet::from([
            VerifierCapability::ObservationHooks,
            VerifierCapability::ResourceContracts,
        ]),
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
                .filter_map(|obligation| {
                    observed_name(&obligation.subject).map(|name| ContractObservationRecord {
                        function: name.to_owned(),
                        hook: name.to_owned(),
                        values: BTreeMap::from([("ok".into(), 1)]),
                    })
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

fn request(
    plugin: &str,
    task: &VerificationTask,
    capability: VerifierCapability,
) -> VerificationRequestV2 {
    VerificationRequestV2 {
        source_effect: None,
        counter_effect: None,
        reactor_program: None,
        teardown_source: None,
        call_use_source: None,
        cleanup_replay_source: None,
        composition_source: None,
        protocol_version: PLUGIN_PROTOCOL_VERSION,
        evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        request_id: format!("{plugin}:{}", &task.task_digest()[..16]),
        plugin_id: PluginId::new(plugin).unwrap(),
        capability,
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
    let request = request(plugin, task, VerifierCapability::ObservationHooks);
    registry.verify_v2(&request, task, Some(&HookReplay))
}

/// The same mint for the second capability (E03-B2's positive control).
fn mint_resources(
    registry: &VerifierRegistry,
    plugin: &str,
    task: &VerificationTask,
) -> VerificationOutcome {
    let request = request(plugin, task, VerifierCapability::ResourceContracts);
    registry.verify_v2(&request, task, Some(&ResourceReplay))
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
fn a_composite_publishes_the_union_of_its_components_premises() {
    // E03-B1 / E03-O1, repaired. Before: unequal but non-conflicting premise
    // sets refused with `QP012` ("... would understate its assumptions"),
    // because the published representative carried one component's
    // `assumptions_used` and would have dropped `callbacks_bounded` from the
    // published result. After: there is no representative to understate — the
    // composite publishes the union, which is the set the merge actually rests
    // on, and a superset of premises is a weaker claim, never a wider one.
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
    // Symmetric in both directions: a subset and a superset merge the same way.
    for order in [[&narrow, &wide], [&wide, &narrow]] {
        let composed = compose_verification(vec![
            mint(&registry, "example.left", order[0]),
            mint(&registry, "example.right", order[1]),
        ]);
        assert_eq!(
            composed.verdict,
            ProposedVerdict::Safe,
            "{:?}",
            composed.diagnostics
        );
        // No component certificate is the merge, so none is published as it.
        assert!(composed.certificate.is_none());
        let composite = composed.composite.as_ref().expect("composite published");
        assert_eq!(*composite.premises(), wide.assumptions);
        // The union is strictly larger than the narrow component's set: the
        // premise that used to be dropped is now published.
        assert!(composite.premises().len() > narrow.assumptions.len());
        assert_eq!(composite.premises().get("callbacks_bounded"), Some(&false));
        assert!(
            composite
                .certificates()
                .iter()
                .any(|certificate| certificate.evidence().assumptions_used
                    != *composite.premises()),
            "at least one component understates the composite, which is why the \
             representative may not be published"
        );
    }
    // Control: the identical premise set is the union of itself, and with one
    // distinct certificate per component the composite still has no
    // representative.
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
        *composed.composite.as_ref().unwrap().premises(),
        narrow.assumptions
    );
    // A premise bound to two different values is a contradiction, not a union:
    // that refusal is unchanged.
    let contradicting = task(
        unit_identity('1'),
        &["flush"],
        BTreeMap::from([("observations_trusted".to_owned(), false)]),
        TaskBounds::observed_runs(),
    );
    let composed = compose_verification(vec![
        mint(&registry, "example.left", &narrow),
        mint(&registry, "example.right", &contradicting),
    ]);
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP012"]);
    assert!(composed.composite.is_none());
}

#[test]
fn a_composite_publishes_the_union_of_its_components_coverage_and_provenance() {
    // E03-B1's other half: coverage, bounds and provenance. Coverage is a
    // union because every identifier in it was discharged by *some*
    // component's core-checked replay, and it is the set the required-set
    // check compares against; provenance is the whole set of contributors.
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
    let required = union(&[&left, &right]);
    let composed = compose_verification_with_inventory(outcomes.clone(), &required);
    assert_eq!(composed.verdict, ProposedVerdict::Safe);
    let composite = composed.composite.as_ref().expect("composite published");

    assert_eq!(*composite.coverage(), required);
    assert_eq!(composite.coverage().len(), 3);
    assert_eq!(*composite.unit(), left.unit);
    assert_eq!(
        composite.plugin_ids().iter().collect::<Vec<_>>(),
        vec![
            &PluginId::new("example.left").unwrap(),
            &PluginId::new("example.right").unwrap()
        ]
    );
    assert_eq!(composite.capabilities().len(), 1);
    assert_eq!(composite.task_digests().len(), 2);
    assert_eq!(composite.obligation_bounds().len(), 3);
    assert_eq!(composite.bounds(), Some(&TaskBounds::observed_runs()));

    // Every component under-reports the merge in coverage and in provenance,
    // which is exactly why publishing one as the composite was a defect.
    for certificate in composite.certificates() {
        assert!(certificate.coverage().is_subset(composite.coverage()));
        assert_ne!(certificate.coverage(), composite.coverage());
        assert!(composite.plugin_ids().contains(certificate.plugin_id()));
    }
    // `compose_certificates` publishes the same composite, not a second view.
    assert_eq!(
        &compose_certificates(&outcomes).expect("composes"),
        composite
    );
}

#[test]
fn a_single_component_composite_still_publishes_its_certificate() {
    // The representative exists exactly when a component *is* the merge: one
    // provider, or the same evidence submitted twice. Anything else has no
    // honest representative and publishes none.
    let registry = registry(&[("example.left", Answer::Honest)]);
    let left = simple_task('1', &["open"]);
    let one = mint(&registry, "example.left", &left);
    let required = union(&[&left]);

    let composed = compose_verification_with_inventory(vec![one.clone()], &required);
    assert_eq!(composed.verdict, ProposedVerdict::Safe);
    assert_eq!(composed.certificate, one.certificate);
    assert_eq!(
        *composed.composite.as_ref().unwrap().coverage(),
        *one.certificate.as_ref().unwrap().coverage()
    );

    let duplicated = compose_verification_with_inventory(vec![one.clone(), one.clone()], &required);
    assert_eq!(duplicated.verdict, ProposedVerdict::Safe);
    assert_eq!(duplicated.certificate, one.certificate);
    assert_eq!(
        duplicated.composite.as_ref().unwrap().certificates().len(),
        2
    );
    assert_eq!(
        *duplicated.composite.as_ref().unwrap().coverage(),
        *one.certificate.as_ref().unwrap().coverage(),
        "a repeat contributes no coverage"
    );
}

#[test]
fn a_composite_publishes_the_tightest_bounds_envelope() {
    // A composite claim is only as strong as its weakest component, so the
    // published envelope is the narrower path space and `observed_runs_only`
    // is the disjunction: one recorded-run component makes the whole
    // composite a recorded-run claim, which the verdict contract requires.
    let registry = registry(&[
        ("example.left", Answer::Honest),
        ("example.right", Answer::Honest),
    ]);
    let observed = simple_task('1', &["open"]);
    let statically_checked = task(
        unit_identity('1'),
        &["flush"],
        observed.assumptions.clone(),
        TaskBounds::single_path(),
    );
    assert!(!statically_checked.bounds.observed_runs_only);
    let composed = compose_verification(vec![
        mint(&registry, "example.left", &observed),
        mint(&registry, "example.right", &statically_checked),
    ]);
    assert_eq!(
        composed.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        composed.diagnostics
    );
    assert!(
        composed
            .composite
            .as_ref()
            .unwrap()
            .bounds()
            .unwrap()
            .observed_runs_only,
        "the composite inherits the recorded-run restriction"
    );
}

#[test]
fn certificates_of_two_capabilities_over_one_unit_merge() {
    // E03-B2's positive control at the merge. Two capabilities over one unit
    // is now a representable composition: the identities agree because a unit
    // identity names the artifact, and the obligation identifiers stay
    // capability-distinct because the capability is hashed into every one.
    let registry = registry(&[
        ("example.left", Answer::Honest),
        ("example.wide", Answer::Honest),
    ]);
    let hooks = simple_task('1', &["open"]);
    let resources = resource_task('1', &["flush"]);
    assert_eq!(hooks.unit, resources.unit);
    let outcomes = vec![
        mint(&registry, "example.left", &hooks),
        mint_resources(&registry, "example.wide", &resources),
    ];
    assert!(
        outcomes
            .iter()
            .all(|outcome| outcome.verdict == ProposedVerdict::Safe),
        "{outcomes:?}"
    );
    let required = hooks
        .coverage_for(VerifierCapability::ObservationHooks)
        .into_iter()
        .chain(resources.coverage_for(VerifierCapability::ResourceContracts))
        .collect::<BTreeSet<_>>();
    assert_eq!(required.len(), 2, "the two capabilities' ids are distinct");
    let composed = compose_verification_with_inventory(outcomes, &required);
    assert_eq!(
        composed.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        composed.diagnostics
    );
    let composite = composed.composite.as_ref().expect("composite published");
    assert_eq!(
        composite.capabilities().iter().copied().collect::<Vec<_>>(),
        vec![
            VerifierCapability::ResourceContracts,
            VerifierCapability::ObservationHooks
        ]
    );
    assert_eq!(*composite.coverage(), required);

    // The identity check is not weakened by this: a different unit still
    // refuses, whichever capabilities the components carry.
    let elsewhere = resource_task('2', &["flush"]);
    let composed = compose_verification(vec![
        mint(&registry, "example.left", &hooks),
        mint_resources(&registry, "example.wide", &elsewhere),
    ]);
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP015"]);
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
        composite: None,
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
        composite: None,
    };
    let composed = compose_verification(vec![forged]);
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP014"]);
}

// ---------------------------------------------------------------------------
// E03-B-O2 — composition is associative under its own output.
// ---------------------------------------------------------------------------

/// Three honest providers over one unit, with disjoint hook coverage.
fn three_components() -> (
    VerifierRegistry,
    VerificationTask,
    VerificationTask,
    VerificationTask,
) {
    let registry = registry(&[
        ("example.left", Answer::Honest),
        ("example.right", Answer::Honest),
        ("example.third", Answer::Honest),
    ]);
    (
        registry,
        simple_task('1', &["open"]),
        simple_task('1', &["close"]),
        simple_task('1', &["flush"]),
    )
}

#[test]
fn composing_a_composite_publishes_the_same_merge_as_composing_the_parts() {
    // E03-B-O2, repaired. Before: a composed outcome carried `certificate:
    // None` for a multi-component merge, so feeding it back refused with
    // `QP014` — fail-closed, but silently non-associative. After: a composed
    // outcome is taken apart into its components, so every grouping of the
    // same components reduces to the same component sequence and publishes the
    // same composite.
    let (registry, left, middle, right) = three_components();
    let a = mint(&registry, "example.left", &left);
    let b = mint(&registry, "example.right", &middle);
    let c = mint(&registry, "example.third", &right);
    let required = union(&[&left, &middle, &right]);
    assert_eq!(required.len(), 3);

    let flat = compose_verification(vec![a.clone(), b.clone(), c.clone()]);
    let left_grouped = compose_verification(vec![
        compose_verification(vec![a.clone(), b.clone()]),
        c.clone(),
    ]);
    let right_grouped = compose_verification(vec![
        a.clone(),
        compose_verification(vec![b.clone(), c.clone()]),
    ]);
    assert_eq!(
        flat.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        flat.diagnostics
    );
    // Associativity in full, not just in the published verdict: the whole
    // outcome — composite, component list, premises, coverage, bounds,
    // provenance and the (absent) representative — is identical.
    assert_eq!(flat, left_grouped);
    assert_eq!(flat, right_grouped);

    let composite = flat.composite.as_ref().expect("composite published");
    assert_eq!(composite.certificates().len(), 3);
    assert_eq!(*composite.coverage(), required);
    assert_eq!(composite.plugin_ids().len(), 3);
    assert!(flat.certificate.is_none());

    // And the required-set check agrees at every grouping, including when the
    // composed outcome is the only component of the final composition.
    for composed in [&flat, &left_grouped, &right_grouped] {
        assert_eq!(
            compose_verification_with_inventory(vec![composed.clone()], &required).verdict,
            ProposedVerdict::Safe
        );
        assert_eq!(
            compose_verification_with_inventory(
                vec![compose_verification(vec![a.clone(), b.clone()]), c.clone()],
                &required
            )
            .verdict,
            ProposedVerdict::Safe
        );
    }
}

#[test]
fn regrouping_cannot_hide_a_conflict_from_the_merge_rules() {
    // Every refusal is decided over the flattened component sequence, so a
    // conflict between components of *different* groups is refused exactly as
    // it is when they are composed flat. Three conflict families, three
    // groupings each.
    let registry = registry(&[
        ("example.left", Answer::Honest),
        ("example.right", Answer::Honest),
        ("example.third", Answer::Honest),
    ]);
    let trusted = simple_task('1', &["open"]);
    let second = simple_task('1', &["close"]);
    let cases: [(&str, VerificationTask); 3] = [
        // A premise bound to two values (QP012).
        (
            "QP012",
            task(
                unit_identity('1'),
                &["flush"],
                BTreeMap::from([("observations_trusted".to_owned(), false)]),
                TaskBounds::observed_runs(),
            ),
        ),
        // The same obligation discharged under two bounds (QP013).
        (
            "QP013",
            task(
                unit_identity('1'),
                &["open"],
                trusted.assumptions.clone(),
                TaskBounds::single_path(),
            ),
        ),
        // A different artifact (QP015).
        ("QP015", simple_task('2', &["flush"])),
    ];
    for (code, conflicting) in cases {
        let a = mint(&registry, "example.left", &trusted);
        let b = mint(&registry, "example.right", &second);
        let c = mint(&registry, "example.third", &conflicting);
        assert_eq!(c.verdict, ProposedVerdict::Safe, "{code}: minted");
        // The conflicting pair is `a` and `c`, and it is refused with `code`
        // wherever the grouping puts them.
        assert_eq!(
            codes(&compose_verification(vec![a.clone(), c.clone()])),
            vec![code]
        );
        for outcomes in [
            vec![a.clone(), b.clone(), c.clone()],
            vec![compose_verification(vec![a.clone(), b.clone()]), c.clone()],
            vec![a.clone(), compose_verification(vec![b.clone(), c.clone()])],
            vec![
                compose_verification(vec![a.clone(), b.clone()]),
                compose_verification(vec![c.clone()]),
            ],
        ] {
            let composed = compose_verification(outcomes);
            assert_eq!(composed.verdict, ProposedVerdict::Unknown, "{code}");
            assert!(composed.composite.is_none(), "{code}");
            assert!(composed.certificate.is_none(), "{code}");
            // Either this composition sees the conflict itself, or an inner
            // group already refused it and contributes an `Unknown` component
            // (`QP011`). A conflict is never dropped by a grouping.
            let observed = codes(&composed);
            assert!(
                observed == vec![code] || observed == vec!["QP011"],
                "{code}: a grouping hid the conflict: {observed:?}"
            );
        }
    }
}

#[test]
fn a_recomposed_composite_is_still_measured_against_the_required_set() {
    // Re-composition carries no coverage a flat composition would not have:
    // an obligation no component discharges is still missing after any number
    // of regroupings.
    let (registry, left, middle, right) = three_components();
    let a = mint(&registry, "example.left", &left);
    let b = mint(&registry, "example.right", &middle);
    let required = union(&[&left, &middle, &right]);
    let pair = compose_verification(vec![a, b]);
    assert_eq!(pair.verdict, ProposedVerdict::Safe);
    let composed = compose_verification_with_inventory(vec![pair.clone()], &required);
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP011"]);
    // The composite it did publish covers exactly the two components' union.
    assert_eq!(
        *pair.composite.as_ref().unwrap().coverage(),
        union(&[&left, &middle])
    );
}

#[test]
fn re_submitting_one_composite_adds_coverage_for_nobody() {
    // Idempotence in the only sense that matters: the merged fields of
    // `[composite, composite]` are the merged fields of `composite`, so a
    // repeat cannot manufacture a discharge.
    let (registry, left, middle, _) = three_components();
    let a = mint(&registry, "example.left", &left);
    let b = mint(&registry, "example.right", &middle);
    let required = union(&[&left, &middle]);
    let once = compose_verification(vec![a, b]);
    let twice = compose_verification(vec![once.clone(), once.clone()]);
    assert_eq!(twice.verdict, ProposedVerdict::Safe);
    let (first, second) = (
        once.composite.as_ref().unwrap(),
        twice.composite.as_ref().unwrap(),
    );
    assert_eq!(second.certificates().len(), 4, "the components are kept");
    assert_eq!(first.coverage(), second.coverage());
    assert_eq!(first.premises(), second.premises());
    assert_eq!(first.bounds(), second.bounds());
    assert_eq!(first.plugin_ids(), second.plugin_ids());
    assert_eq!(first.obligation_bounds(), second.obligation_bounds());
    assert!(second.representative().is_none());
    assert_eq!(
        compose_verification_with_inventory(vec![twice], &required).verdict,
        ProposedVerdict::Safe
    );
}

#[test]
fn only_a_safe_outcome_contributes_its_composite() {
    // Mutation rows. The verdict gates run before any component is taken
    // apart, so a composite attached to a non-`Safe` outcome never enters a
    // merge, and a `Safe` outcome carrying neither a certificate nor a
    // composite is still `QP014`.
    let (registry, left, middle, _) = three_components();
    let a = mint(&registry, "example.left", &left);
    let b = mint(&registry, "example.right", &middle);
    let honest = compose_verification(vec![a.clone(), b]);
    let composite = honest.composite.clone().expect("composite published");

    for (verdict, code) in [
        (ProposedVerdict::Unknown, "QP011"),
        (ProposedVerdict::Unsafe, "QP010"),
    ] {
        let smuggled = VerificationOutcome {
            verdict,
            diagnostics: Vec::new(),
            certificate: None,
            composite: Some(composite.clone()),
        };
        let composed = compose_verification(vec![a.clone(), smuggled]);
        assert_eq!(composed.verdict, ProposedVerdict::Unknown);
        assert_eq!(codes(&composed), vec![code]);
        assert!(composed.composite.is_none());
    }

    let empty = VerificationOutcome {
        verdict: ProposedVerdict::Safe,
        diagnostics: Vec::new(),
        certificate: None,
        composite: None,
    };
    let composed = compose_verification(vec![honest, empty]);
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(codes(&composed), vec!["QP014"]);
}

// ---------------------------------------------------------------------------
// E03-B-O3 — the bounds intersection, kind by kind.
// ---------------------------------------------------------------------------

#[test]
fn the_bounds_envelope_is_the_tightest_of_every_kind() {
    // The algebra of `tighter_bounds`, exercised through the real merge with
    // real minted certificates. `crates/qcore-verification/tests/
    // composition_bounds.rs` runs the `BoundedPaths` row against two *derived*
    // claims; this table covers the kinds and the directions.
    let registry = registry(&[
        ("example.left", Answer::Honest),
        ("example.right", Answer::Honest),
    ]);
    let bounds =
        |path_exploration, schedules_explored, observed_runs_only, max_operations| TaskBounds {
            path_exploration,
            schedules_explored,
            observed_runs_only,
            max_operations,
        };
    let cases = [
        (
            "two path counts take the minimum",
            bounds(PathBounds::BoundedPaths { paths: 5 }, None, false, None),
            bounds(PathBounds::BoundedPaths { paths: 3 }, None, false, None),
            bounds(PathBounds::BoundedPaths { paths: 3 }, None, false, None),
        ),
        (
            "a single path absorbs any count",
            bounds(PathBounds::SinglePath, None, false, None),
            bounds(PathBounds::BoundedPaths { paths: 3 }, None, false, None),
            bounds(PathBounds::SinglePath, None, false, None),
        ),
        (
            "two schedule counts take the minimum",
            bounds(PathBounds::SinglePath, Some(9), false, None),
            bounds(PathBounds::SinglePath, Some(4), false, None),
            bounds(PathBounds::SinglePath, Some(4), false, None),
        ),
        (
            "an absent schedule count is not applicable, never unbounded",
            bounds(PathBounds::SinglePath, None, false, None),
            bounds(PathBounds::SinglePath, Some(4), false, None),
            bounds(PathBounds::SinglePath, Some(4), false, None),
        ),
        (
            "two operation counts take the minimum",
            bounds(PathBounds::SinglePath, None, false, Some(64)),
            bounds(PathBounds::SinglePath, None, false, Some(16)),
            bounds(PathBounds::SinglePath, None, false, Some(16)),
        ),
        (
            "an absent operation count does not relax the composite",
            bounds(PathBounds::SinglePath, None, false, Some(64)),
            bounds(PathBounds::SinglePath, None, false, None),
            bounds(PathBounds::SinglePath, None, false, Some(64)),
        ),
        (
            "recorded-run restriction is the disjunction",
            bounds(PathBounds::SinglePath, None, true, None),
            bounds(PathBounds::SinglePath, None, false, None),
            bounds(PathBounds::SinglePath, None, true, None),
        ),
    ];
    let assumptions = BTreeMap::from([("observations_trusted".to_owned(), true)]);
    for (label, left, right, expected) in cases {
        let left_task = task(unit_identity('1'), &["open"], assumptions.clone(), left);
        let right_task = task(unit_identity('1'), &["flush"], assumptions.clone(), right);
        // Both orders: the envelope does not depend on which component is seen
        // first.
        for order in [[&left_task, &right_task], [&right_task, &left_task]] {
            let composed = compose_verification(vec![
                mint(&registry, "example.left", order[0]),
                mint(&registry, "example.right", order[1]),
            ]);
            assert_eq!(
                composed.verdict,
                ProposedVerdict::Safe,
                "{label}: {:?}",
                composed.diagnostics
            );
            assert_eq!(
                composed.composite.as_ref().unwrap().bounds(),
                Some(&expected),
                "{label}"
            );
        }
    }
}

#[test]
fn a_component_without_a_task_leaves_the_composite_with_no_bounds() {
    // `None` bounds are "this composite states none", never "unbounded": a
    // component carrying no task cannot be given one by the merge. Pinned
    // here because the envelope rule above only holds when every component
    // states its own.
    let registry = registry(&[("example.left", Answer::Honest)]);
    let left = simple_task('1', &["open"]);
    let with_task = mint(&registry, "example.left", &left);
    assert!(
        with_task
            .certificate
            .as_ref()
            .expect("safe result carries a certificate")
            .task()
            .is_some()
    );
    assert!(
        compose_verification(vec![with_task])
            .composite
            .as_ref()
            .unwrap()
            .bounds()
            .is_some()
    );
}
