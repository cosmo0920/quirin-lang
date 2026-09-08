//! E03 — bounded multi-provider composition on the production dispatch route.
//!
//! The host's `crates/qcore-verifier-host/tests/composition_relation.rs`
//! audits the composition algebra itself. This file runs two independent
//! providers through one real `verify scope`, with the production
//! `ContractObservationReplay` checker, the real registry and the real
//! `dispatch_scopes` orchestration, and pins what the resulting merge does.
//!
//! Nothing here promotes any raw unit: the fixture is a synthetic QCore source
//! program, and observation replay remains a recorded-run claim under the
//! declared `observations_trusted` premise.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use qcore_frontend::frontend::{SourceProgram, parse_source_program};
use qcore_schemas::obligation::ObligationProperty;
use qcore_schemas::witness::{
    ContractObservationRecord, ContractObservationWitness, NativeRunBinding, ObservedRun,
};
use qcore_verification::obligation_derivation::{DerivationInputs, derive_task};
use qcore_verification::verify_dispatch::dispatch_scopes;
use qcore_verifier_host::*;

const UNIT: &str = "unit.qc";

/// One unit, two contract hooks, one scope naming two independent providers.
const SOURCE: &str = "\
extern fn resize() -> unit contract { requires size > 0; ensures normal live == 1; };
extern fn grow() -> unit contract { requires size > 0; ensures normal live == 1; };
verify scope observed {
  plugin example.left profile default capabilities { observation_hooks };
  plugin example.right profile default capabilities { observation_hooks };
}
fn main() -> i32 { return 7; }
";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Answer {
    Honest,
    Unknown,
    Alleged,
    Fail,
    /// Claims the *other* provider's hook instead of its own.
    Foreign,
}

fn manifest(id: &str, hook: &str) -> PluginManifest {
    PluginManifest {
        plugin_id: PluginId::new(id).unwrap(),
        package_version: "0.1.0".into(),
        protocol_versions: BTreeSet::from([PLUGIN_PROTOCOL_VERSION]),
        evidence_versions: BTreeSet::from([PLUGIN_EVIDENCE_VERSION, PLUGIN_EVIDENCE_VERSION_V2]),
        entrypoint: PluginEntrypoint::BuiltIn,
        package_digest: format!("builtin:{id}"),
        targets: vec![SupportedTarget {
            library: "conformance".into(),
            revisions: BTreeSet::from(["source".into()]),
            profiles: BTreeSet::from(["default".into()]),
            dialects: BTreeSet::from(["qcore".into()]),
            platforms: BTreeSet::from([std::env::consts::OS.into()]),
            backends: BTreeSet::from(["native".into()]),
        }],
        capabilities: BTreeSet::from([VerifierCapability::ObservationHooks]),
        required_foreign_summaries: BTreeSet::new(),
        required_observation_hooks: BTreeSet::from([hook.to_owned()]),
        filesystem_inputs: BTreeSet::new(),
        network_access: false,
        limits: PluginLimits::default(),
        trust_class: PluginTrustClass::FirstParty,
        certificate_checker: "qcore.contract_observation.v2".into(),
        lean_theorem_family: "QCore.VerifierPlugin".into(),
    }
}

struct Fixture {
    manifest: PluginManifest,
    hook: String,
    other_hook: String,
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
        let hook = if self.answer == Answer::Foreign {
            &self.other_hook
        } else {
            &self.hook
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
                        run_id: format!("{}-run", request.plugin_id),
                        binding: NativeRunBinding {
                            emitted_source_digest: "a".repeat(64),
                            executable_digest: "b".repeat(64),
                            compiler_invocation: "cc fixture.c".into(),
                            exit_status: 0,
                            seed_or_input: "size=4".into(),
                            profile: "default".into(),
                            instrumentation_version: "fixture-v1".into(),
                        },
                        observations: vec![ContractObservationRecord {
                            function: hook.clone(),
                            hook: hook.clone(),
                            values: BTreeMap::from([("size".into(), 4), ("live".into(), 1)]),
                        }],
                    }],
                }),
            }),
        ))
    }
}

fn registry(left: Answer, right: Answer) -> VerifierRegistry {
    let mut registry = VerifierRegistry::default();
    for (id, hook, other, answer) in [
        ("example.left", "resize", "grow", left),
        ("example.right", "grow", "resize", right),
    ] {
        registry
            .register(Arc::new(Fixture {
                manifest: manifest(id, hook),
                hook: hook.to_owned(),
                other_hook: other.to_owned(),
                answer,
            }))
            .expect("fixture manifests are valid");
    }
    registry
}

fn program() -> SourceProgram {
    parse_source_program(SOURCE).expect("fixture parses")
}

fn dispatch(left: Answer, right: Answer) -> qcore_verification::verify_dispatch::DispatchReport {
    let registry = registry(left, right);
    let parsed = program();
    let resolved = registry
        .resolve_source_scopes(&parsed)
        .expect("both plugins resolve");
    assert_eq!(
        resolved[0].providers.len(),
        2,
        "the scope names two providers"
    );
    dispatch_scopes(&parsed, UNIT, &resolved, &registry)
}

fn scope_codes(report: &qcore_verification::verify_dispatch::DispatchReport) -> Vec<&'static str> {
    report
        .diagnostics()
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn two_providers_discharge_one_scopes_whole_obligation_inventory() {
    let report = dispatch(Answer::Honest, Answer::Honest);
    assert!(report.scopes[0].verified(), "{:?}", report.diagnostics());
    assert_eq!(report.scopes[0].capabilities.len(), 2);
    assert!(
        report.scopes[0].capabilities.iter().all(
            |dispatch| dispatch.dispatched && dispatch.outcome.verdict == ProposedVerdict::Safe
        )
    );

    // The core's required set is the union over both providers' tasks. Each
    // provider contributes its own hook obligation and its own coverage
    // obligation: four members. (Before E03-B3's repair the unit-scoped
    // `ObservationCoverageComplete` obligation had one core-computed
    // identifier for both providers and this union had three members, sound
    // only because every component must be `Safe`.)
    let required = report.scopes[0]
        .capabilities
        .iter()
        .flat_map(|dispatch| {
            dispatch
                .task
                .coverage_for(VerifierCapability::ObservationHooks)
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(required.len(), 4);
    let covered = report.scopes[0]
        .capabilities
        .iter()
        .flat_map(|dispatch| {
            dispatch
                .outcome
                .certificate
                .as_ref()
                .expect("safe result carries a certificate")
                .coverage()
                .iter()
                .cloned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(covered, required);
    assert_eq!(
        compose_verification_with_inventory(
            report.scopes[0]
                .capabilities
                .iter()
                .map(|dispatch| dispatch.outcome.clone())
                .collect(),
            &required,
        )
        .verdict,
        ProposedVerdict::Safe
    );
    // E03-B1. Two distinct components, so no single certificate is the merge:
    // the published result carries the composite and no representative.
    assert!(report.scopes[0].composed.certificate.is_none());
    let composite = report.scopes[0]
        .composed
        .composite
        .as_ref()
        .expect("composed composite");
    assert_eq!(composite.certificates().len(), 2);
    assert_eq!(*composite.coverage(), required);
    assert_eq!(
        composite.premises().get("observations_trusted"),
        Some(&true)
    );
    assert!(composite.bounds().unwrap().observed_runs_only);
    assert_eq!(composite.plugin_ids().len(), 2);
    assert_eq!(
        composite.capabilities().iter().copied().collect::<Vec<_>>(),
        vec![VerifierCapability::ObservationHooks]
    );
    assert_eq!(composite.task_digests().len(), 2);
    // Every component's own coverage is a strict subset of the composite's, so
    // publishing one would have understated the merge.
    for certificate in composite.certificates() {
        assert!(certificate.coverage().is_subset(composite.coverage()));
        assert_ne!(certificate.coverage(), composite.coverage());
    }
}

#[test]
fn one_missing_provider_leaves_its_obligations_undischarged() {
    for answer in [Answer::Unknown, Answer::Fail, Answer::Foreign] {
        let report = dispatch(Answer::Honest, answer);
        assert!(!report.scopes[0].verified());
        assert_eq!(scope_codes(&report), vec!["Q701"]);
        assert_eq!(report.scopes[0].composed.verdict, ProposedVerdict::Unknown);
        let right = report.scopes[0]
            .capabilities
            .iter()
            .find(|dispatch| dispatch.plugin_id == "example.right")
            .expect("the failing provider was still dispatched");
        assert_eq!(right.outcome.verdict, ProposedVerdict::Unknown);
        assert!(right.outcome.certificate.is_none());
        // Its obligations stay in the core's inventory rather than vanishing.
        assert!(
            !right
                .task
                .coverage_for(VerifierCapability::ObservationHooks)
                .is_empty()
        );
        // A provider answering with the other provider's hook is refused by the
        // core's own replay, not silently credited to the honest provider.
        if answer == Answer::Foreign {
            assert!(
                right
                    .outcome
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == "QP009")
            );
        }
    }
}

#[test]
fn an_allegation_by_one_provider_is_not_a_scope_wide_checked_violation() {
    let report = dispatch(Answer::Honest, Answer::Alleged);
    assert!(!report.scopes[0].verified());
    // Q703 names the unverifiable allegation; the scope verdict stays Unknown
    // and no certificate is published.
    let mut codes = scope_codes(&report);
    codes.sort_unstable();
    assert_eq!(codes, vec!["Q701", "Q703"]);
    assert_eq!(report.scopes[0].composed.verdict, ProposedVerdict::Unknown);
    assert!(report.scopes[0].composed.certificate.is_none());
    let alleging = report.scopes[0]
        .capabilities
        .iter()
        .find(|dispatch| dispatch.plugin_id == "example.right")
        .unwrap();
    assert_eq!(alleging.outcome.verdict, ProposedVerdict::Unknown);
    assert!(
        alleging
            .outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "QP010")
    );
}

#[test]
fn a_scope_provider_absent_from_the_registry_is_never_vacuously_verified() {
    // `resolve_source_scopes` rejects an uninstalled plugin before dispatch;
    // this pins the dispatcher's own defensive branch for a hand-built scope.
    let registry = registry(Answer::Honest, Answer::Honest);
    let parsed = program();
    let mut resolved = registry.resolve_source_scopes(&parsed).unwrap();
    resolved[0].providers.push(ResolvedVerifierUse {
        plugin_id: PluginId::new("example.absent").unwrap(),
        package_version: "0.1.0".into(),
        profile: "default".into(),
        capabilities: BTreeSet::from([VerifierCapability::ObservationHooks]),
    });
    let report = dispatch_scopes(&parsed, UNIT, &resolved, &registry);
    assert!(!report.scopes[0].verified());
    assert_eq!(scope_codes(&report), vec!["Q701"]);
    assert!(report.diagnostics()[0].message.contains("example.absent"));
}

#[test]
fn an_empty_scope_is_unverified_rather_than_vacuously_safe() {
    let registry = registry(Answer::Honest, Answer::Honest);
    let parsed = program();
    let report = dispatch_scopes(
        &parsed,
        UNIT,
        &[ResolvedVerifierScope {
            name: "observed".into(),
            providers: Vec::new(),
        }],
        &registry,
    );
    assert!(!report.scopes[0].verified());
    assert_eq!(scope_codes(&report), vec!["Q701", "Q701"]);
    assert_eq!(report.scopes[0].composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(report.scopes[0].composed.diagnostics[0].code, "QP016");
}

fn derivation_inputs<'a>(
    parsed: &'a SourceProgram,
    verifier: Option<&str>,
    hooks: &[&str],
) -> DerivationInputs<'a> {
    DerivationInputs {
        translation_unit: UNIT,
        program: parsed,
        target: TargetProfile {
            library: "conformance".into(),
            revision: "source".into(),
            profile: "default".into(),
            dialect: "qcore".into(),
            platform: std::env::consts::OS.into(),
            backend: "native".into(),
        },
        contract_sources: BTreeMap::from([(UNIT.to_owned(), parsed.source().to_owned())]),
        required_observation_hooks: hooks
            .iter()
            .map(|hook| (*hook).to_owned())
            .collect::<BTreeSet<_>>(),
        assumptions: BTreeMap::new(),
        verifier: verifier.map(ToOwned::to_owned),
    }
}

#[test]
fn one_program_has_one_unit_identity_across_capabilities() {
    // E03-B2, repaired. Before: `build_digest` folded in the capability's
    // semantic profile, so two capabilities of one source program certified
    // different `UnitIdentity` values and `compose_verification` refused every
    // multi-capability scope with QP015 — a completeness cost with no
    // soundness content. After: one program is one unit, and the capability
    // separation lives where it is checked.
    let parsed = program();
    let inputs = derivation_inputs(&parsed, None, &["resize"]);
    let observation = derive_task(&inputs, VerifierCapability::ObservationHooks);
    let lifecycle = derive_task(&inputs, VerifierCapability::ChunkLifecycle);
    assert_eq!(observation.task.unit, lifecycle.task.unit);

    // What used to be carried by the unit identity is still carried, and is
    // still what binds a witness: the profile, the task digest and every
    // obligation identifier stay capability-distinct.
    assert_ne!(
        observation.task.semantic_profile,
        lifecycle.task.semantic_profile
    );
    assert_ne!(observation.task.task_digest(), lifecycle.task.task_digest());
    let observation_ids = observation
        .task
        .coverage_for(VerifierCapability::ObservationHooks);
    assert!(!observation_ids.is_empty());
    assert!(
        observation_ids
            .iter()
            .all(|id| id.starts_with("observation_hooks:"))
    );
    assert!(
        observation
            .task
            .coverage_for(VerifierCapability::ChunkLifecycle)
            .is_empty()
    );

    // Independently, the lifecycle derivation is incomplete here, so it can
    // never reach `Safe` whatever a provider returns: this repair represents a
    // multi-capability composition, it does not promote one.
    assert!(
        lifecycle
            .task
            .is_incomplete_for(VerifierCapability::ChunkLifecycle)
    );
}

#[test]
fn a_real_artifact_difference_still_refuses_the_merge() {
    // The other half of E03-B2: dropping the capability from the build digest
    // must not drop the identity check. A different source program and a
    // different translation unit each still change the unit identity.
    let parsed = program();
    let other = parse_source_program(&format!("{SOURCE}fn spare() -> i32 {{ return 1; }}\n"))
        .expect("fixture parses");
    let mine = derive_task(
        &derivation_inputs(&parsed, None, &["resize"]),
        VerifierCapability::ObservationHooks,
    );
    let theirs = derive_task(
        &derivation_inputs(&other, None, &["resize"]),
        VerifierCapability::ObservationHooks,
    );
    assert_ne!(mine.task.unit.source_digest, theirs.task.unit.source_digest);
    assert_ne!(mine.task.unit, theirs.task.unit);

    let mut renamed = derivation_inputs(&parsed, None, &["resize"]);
    renamed.translation_unit = "other.qc";
    let elsewhere = derive_task(&renamed, VerifierCapability::ObservationHooks);
    assert_ne!(
        mine.task.unit.build_digest, elsewhere.task.unit.build_digest,
        "the build digest still names the translation unit"
    );
}

#[test]
fn two_providers_derive_two_distinct_coverage_obligations() {
    // E03-B3, repaired. Before: `ObservationCoverageComplete` has subject
    // `Unit`, so both providers derived *one* identifier for two different
    // claims, and the two-provider required set had three members. It was
    // sound only because every component must be `Safe`. After: the identifier
    // is keyed by the provider and by the resolved hook denominator the replay
    // decides it against, so the required set has four.
    let parsed = program();
    let coverage = |verifier: &str, hook: &str| {
        derive_task(
            &derivation_inputs(&parsed, Some(verifier), &[hook]),
            VerifierCapability::ObservationHooks,
        )
        .task
        .obligations
        .into_iter()
        .filter(|obligation| obligation.property == ObligationProperty::ObservationCoverageComplete)
        .map(|obligation| obligation.obligation_id)
        .collect::<Vec<_>>()
    };
    let left = coverage("example.left", "resize");
    let right = coverage("example.right", "grow");
    assert_eq!(left.len(), 1);
    assert_eq!(right.len(), 1);
    assert_ne!(left, right, "distinct providers, distinct obligations");
    // Both halves of the key matter, and each alone changes the identifier.
    assert_ne!(left, coverage("example.right", "resize"));
    assert_ne!(left, coverage("example.left", "grow"));
    // The subject is unchanged, so the repair is in the identifier alone.
    assert_eq!(left, coverage("example.left", "resize"));
}

#[test]
fn one_providers_certificate_cannot_discharge_anothers_coverage_obligation() {
    // The adversarial direction of E03-B3: the honest provider's certificate
    // must not cover the silent provider's coverage obligation. Pre-repair the
    // shared identifier left exactly one obligation uncovered here; now two
    // are, and the count is pinned so the property cannot drift back.
    let report = dispatch(Answer::Honest, Answer::Unknown);
    assert!(!report.scopes[0].verified());
    let required = report.scopes[0]
        .capabilities
        .iter()
        .flat_map(|dispatch| {
            dispatch
                .task
                .coverage_for(VerifierCapability::ObservationHooks)
        })
        .collect::<BTreeSet<_>>();
    let covered = report.scopes[0]
        .capabilities
        .iter()
        .filter_map(|dispatch| dispatch.outcome.certificate.as_ref())
        .flat_map(|certificate| certificate.coverage().iter().cloned())
        .collect::<BTreeSet<_>>();
    assert_eq!(required.len(), 4);
    assert_eq!(covered.len(), 2);
    assert_eq!(required.difference(&covered).count(), 2);
}

#[test]
fn an_absent_provider_refuses_the_composed_verdict_itself() {
    // E03-B4, repaired. Before: `dispatch_scope` skipped an unregistered
    // provider before deriving its obligations, so they never entered the
    // required set and `composed.verdict` alone could read `Safe` while
    // `verified()` was false — masked only by `Q700` at resolution and by
    // every consumer reading `diagnostics()`. After: absence is a refusal in
    // the composed verdict, under the registry's own `QP002`.
    let registry = registry(Answer::Honest, Answer::Honest);
    let parsed = program();
    let mut resolved = registry.resolve_source_scopes(&parsed).unwrap();

    // Control: the same scope without the absent provider composes `Safe`, so
    // the refusal below is attributable to the absence alone.
    let control = dispatch_scopes(&parsed, UNIT, &resolved, &registry);
    assert_eq!(control.scopes[0].composed.verdict, ProposedVerdict::Safe);

    resolved[0].providers.push(ResolvedVerifierUse {
        plugin_id: PluginId::new("example.absent").unwrap(),
        package_version: "0.1.0".into(),
        profile: "default".into(),
        capabilities: BTreeSet::from([VerifierCapability::ObservationHooks]),
    });
    let report = dispatch_scopes(&parsed, UNIT, &resolved, &registry);
    assert!(!report.scopes[0].verified());
    assert_eq!(report.scopes[0].composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(report.scopes[0].composed.diagnostics[0].code, "QP002");
    assert!(
        report.scopes[0].composed.diagnostics[0]
            .message
            .contains("example.absent")
    );
    assert!(report.scopes[0].composed.certificate.is_none());
    assert!(report.scopes[0].composed.composite.is_none());
    // The providers that did answer are still dispatched and still reported.
    assert_eq!(report.scopes[0].capabilities.len(), 2);
    assert!(
        report.scopes[0]
            .capabilities
            .iter()
            .all(|dispatch| dispatch.outcome.verdict == ProposedVerdict::Safe)
    );
}
