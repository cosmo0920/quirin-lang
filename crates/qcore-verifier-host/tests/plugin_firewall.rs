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

//! Card W45-USER-CONTRACT-PLUGINS — the checker-plugin firewall at the host:
//! every row of the classification table in `plugin_firewall`, through the
//! real registry. Nothing here fabricates a `CoreCertificate`: the certified
//! row is minted by the registry's own witness replay.
//!
//! The providers are in-process test doubles of the provider trait, as in the
//! host's other firewall tests; the audit route runs checker plugins only as
//! verified process packages, and its end-to-end tests drive real processes
//! (`crates/qcore-audit/tests/user_contract_plugins.rs`). The witness and
//! counterexample checkers are small test doubles of trusted-core code: they
//! decide against the core's own obligation list, never against provider data.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use qcore_schemas::verdict::Verdict;
use qcore_verifier_host::*;

const MODULE: &str = "quirin-provider-acme:acme/pool.qv";

fn task() -> VerificationTask {
    let mut task = VerificationTask {
        task_version: VERIFICATION_TASK_VERSION,
        unit: UnitIdentity {
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
            translation_unit: MODULE.into(),
        },
        target: TargetProfile {
            library: "acme".into(),
            revision: "0.1.0".into(),
            profile: "user-contract".into(),
            dialect: "qcore-resource-v1".into(),
            platform: "any".into(),
            backend: "quirin-audit".into(),
        },
        semantic_profile: "quirin.user-contract-check.v1".into(),
        contract_sources: BTreeMap::from([(MODULE.to_owned(), "c".repeat(64))]),
        bounds: TaskBounds::single_path(),
        assumptions: BTreeMap::new(),
        evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        obligations: Vec::new(),
        derivation_incomplete: Vec::new(),
    };
    for symbol in ["pool_get", "pool_put"] {
        let subject = ObligationSubject::Function {
            symbol: symbol.into(),
            translation_unit: MODULE.into(),
        };
        let obligation_id = task.obligation_id(
            VerifierCapability::ResourceContracts,
            ObligationProperty::ParameterModeRespected,
            &subject,
        );
        task.obligations.push(Obligation {
            obligation_id,
            capability: VerifierCapability::ResourceContracts,
            property: ObligationProperty::ParameterModeRespected,
            subject,
            scope: ObligationScope::Functions {
                symbols: BTreeSet::from([symbol.to_owned()]),
            },
            bounds: TaskBounds::single_path(),
            assumptions: BTreeMap::new(),
            evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        });
    }
    task.obligations.sort();
    task.validate().unwrap();
    task
}

fn symbol(obligation: &Obligation) -> &str {
    match &obligation.subject {
        ObligationSubject::Function { symbol, .. } => symbol,
        _ => "",
    }
}

/// Discharges exactly the obligations whose function the witness observed.
struct Replay;

impl WitnessChecker for Replay {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        let SemanticWitness::ContractObservation(witness) = witness else {
            return Err("not an observation witness".into());
        };
        let observed: BTreeSet<&str> = witness
            .runs
            .iter()
            .flat_map(|run| run.observations.iter().map(|o| o.function.as_str()))
            .collect();
        Ok(task
            .obligations
            .iter()
            .filter(|obligation| observed.contains(symbol(obligation)))
            .map(|obligation| obligation.obligation_id.clone())
            .collect())
    }
}

/// Validates a counterexample naming a function of the core's inventory.
struct Counterexample;

impl CounterexampleChecker for Counterexample {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        Replay.check(task, witness)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// `Safe` with a witness observing every function.
    Certified,
    /// `Safe` with a witness observing one function of two.
    Partial,
    /// `Safe` with no evidence.
    Bare,
    /// `Safe` with evidence bound to another unit.
    Misbound,
    /// `Unsafe` with a witness observing `pool_get`.
    Witnessed,
    /// `Unsafe` with no witness.
    Alleged,
    /// `Unsafe` with a witness naming nothing the core derived.
    Empty,
    Unknown,
    Timeout,
    Crash,
    /// A response for another request.
    WrongRequest,
}

struct Plugin {
    manifest: PluginManifest,
    answer: Answer,
}

fn manifest() -> PluginManifest {
    PluginManifest {
        plugin_id: PluginId::new("acme.pool_check").unwrap(),
        package_version: "0.1.0".into(),
        protocol_versions: BTreeSet::from([PLUGIN_PROTOCOL_VERSION]),
        evidence_versions: BTreeSet::from([PLUGIN_EVIDENCE_VERSION_V2]),
        entrypoint: PluginEntrypoint::BuiltIn,
        package_digest: "builtin:acme.pool_check".into(),
        targets: vec![SupportedTarget {
            library: "acme".into(),
            revisions: BTreeSet::from(["*".into()]),
            profiles: BTreeSet::from(["user-contract".into()]),
            dialects: BTreeSet::from(["qcore-resource-v1".into()]),
            platforms: BTreeSet::from(["*".into()]),
            backends: BTreeSet::from(["quirin-audit".into()]),
        }],
        capabilities: BTreeSet::from([VerifierCapability::ResourceContracts]),
        required_foreign_summaries: BTreeSet::new(),
        required_observation_hooks: BTreeSet::new(),
        filesystem_inputs: BTreeSet::new(),
        network_access: false,
        limits: PluginLimits::default(),
        trust_class: PluginTrustClass::ThirdParty,
        certificate_checker: "acme.pool_check.replay".into(),
        lean_theorem_family: "QCore.VerdictSemigroup".into(),
    }
}

fn witness(request: &VerificationRequestV2, functions: &[&str]) -> SemanticWitness {
    SemanticWitness::ContractObservation(ContractObservationWitness {
        task_digest: request.task_digest.clone(),
        runs: vec![ObservedRun {
            run_id: "run".into(),
            binding: NativeRunBinding {
                emitted_source_digest: "a".repeat(64),
                executable_digest: "b".repeat(64),
                compiler_invocation: "cc pool.c".into(),
                exit_status: 0,
                seed_or_input: "fixture".into(),
                profile: "default".into(),
                instrumentation_version: "v1".into(),
            },
            observations: functions
                .iter()
                .map(|function| ContractObservationRecord {
                    function: (*function).into(),
                    hook: (*function).into(),
                    values: BTreeMap::from([("ok".into(), 1)]),
                })
                .collect(),
        }],
    })
}

impl VerifierProvider for Plugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn verify(&self, _: &VerificationRequest) -> Result<VerificationResponse, ProviderFailure> {
        Err(ProviderFailure::Protocol("evidence v2 only".into()))
    }

    fn verify_v2(
        &self,
        request: &VerificationRequestV2,
    ) -> Result<VerificationResponseV2, ProviderFailure> {
        let evidence = |functions: &[&str], unit: UnitIdentity| ProposedEvidenceV2 {
            evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
            unit,
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
            witness: witness(request, functions),
        };
        let (verdict, evidence) = match self.answer {
            Answer::Certified => (
                ProposedVerdict::Safe,
                Some(evidence(&["pool_get", "pool_put"], request.unit.clone())),
            ),
            Answer::Partial => (
                ProposedVerdict::Safe,
                Some(evidence(&["pool_get"], request.unit.clone())),
            ),
            Answer::Bare => (ProposedVerdict::Safe, None),
            Answer::Misbound => {
                let mut unit = request.unit.clone();
                unit.source_digest = "d".repeat(64);
                (
                    ProposedVerdict::Safe,
                    Some(evidence(&["pool_get", "pool_put"], unit)),
                )
            }
            Answer::Witnessed => (
                ProposedVerdict::Unsafe,
                Some(evidence(&["pool_get"], request.unit.clone())),
            ),
            Answer::Alleged => (ProposedVerdict::Unsafe, None),
            Answer::Empty => (
                ProposedVerdict::Unsafe,
                Some(evidence(&["unrelated"], request.unit.clone())),
            ),
            Answer::Unknown => (ProposedVerdict::Unknown, None),
            Answer::Timeout => return Err(ProviderFailure::Timeout),
            Answer::Crash => return Err(ProviderFailure::Crashed(Some(3))),
            Answer::WrongRequest => {
                return Ok(VerificationResponseV2 {
                    protocol_version: request.protocol_version,
                    request_id: "another".into(),
                    plugin_id: request.plugin_id.clone(),
                    verdict: ProposedVerdict::Safe,
                    diagnostics: vec![],
                    evidence: None,
                });
            }
        };
        Ok(VerificationResponseV2 {
            protocol_version: request.protocol_version,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict,
            diagnostics: vec![PluginDiagnostic {
                code: "acme.pool_check.note".into(),
                message: "fixture".into(),
                fact_id: None,
            }],
            evidence,
        })
    }
}

fn request(task: &VerificationTask) -> VerificationRequestV2 {
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
        request_id: "acme.pool_check:1".into(),
        plugin_id: PluginId::new("acme.pool_check").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: task.target.clone(),
        unit: task.unit.clone(),
        assumptions: task.assumptions.clone(),
        task_digest: task.task_digest(),
        obligations: task.obligations.clone(),
        facts: Vec::new(),
    }
}

fn claim(
    answer: Answer,
    checker: Option<&dyn WitnessChecker>,
    counterexample: Option<&dyn CounterexampleChecker>,
) -> PluginClaim {
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(Plugin {
            manifest: manifest(),
            answer,
        }))
        .unwrap();
    let task = task();
    registry.verify_plugin_claim(&request(&task), &task, checker, counterexample)
}

fn verdict(claim: &PluginClaim) -> Verdict {
    claim.verdict("acme.pool_check").verdict()
}

#[test]
fn only_a_registry_certificate_makes_a_plugin_safe() {
    let certified = claim(Answer::Certified, Some(&Replay), None);
    assert!(
        matches!(certified, PluginClaim::CertifiedSafe(_)),
        "{certified:?}"
    );
    assert_eq!(verdict(&certified), Verdict::Safe);
    let PluginClaim::CertifiedSafe(certificate) = certified else {
        unreachable!()
    };
    assert_eq!(
        certificate.coverage(),
        &task().coverage_for(VerifierCapability::ResourceContracts)
    );
}

#[test]
fn an_uncertified_safe_claim_is_pending() {
    for (answer, checker, code) in [
        (Answer::Bare, Some(&Replay as &dyn WitnessChecker), "QP008"),
        (Answer::Certified, None, "QP016"),
    ] {
        let claim = claim(answer, checker, None);
        assert!(
            matches!(claim, PluginClaim::UncertifiedSafe { .. }),
            "{claim:?}"
        );
        assert_eq!(claim.code(), Some(code));
        let reasoned = claim.verdict("acme.pool_check");
        assert_eq!(reasoned.verdict(), Verdict::Pending);
        assert_eq!(reasoned.reasons()[0].code, "plugin-claim-uncertified");
        assert_eq!(
            qcore_schemas::reason::ReasonCategory::classify(&reasoned.reasons()[0].detail),
            Ok(qcore_schemas::reason::ReasonCategory::PluginClaimUncertified)
        );
    }
}

#[test]
fn a_checked_and_refuted_or_misbound_certificate_is_unknown() {
    for answer in [Answer::Partial, Answer::Misbound] {
        let claim = claim(answer, Some(&Replay), None);
        assert!(matches!(claim, PluginClaim::Failure { .. }), "{claim:?}");
        assert_eq!(claim.code(), Some("QP009"));
        assert_eq!(verdict(&claim), Verdict::Unknown);
    }
}

#[test]
fn an_unsafe_claim_is_pending_unless_a_counterexample_checker_validates_it() {
    for (answer, counterexample) in [
        (
            Answer::Alleged,
            Some(&Counterexample as &dyn CounterexampleChecker),
        ),
        (Answer::Witnessed, None),
    ] {
        let claim = claim(answer, Some(&Replay), counterexample);
        assert!(
            matches!(claim, PluginClaim::UnvalidatedUnsafe { .. }),
            "{claim:?}"
        );
        assert_eq!(verdict(&claim), Verdict::Pending);
    }
    let validated = claim(Answer::Witnessed, None, Some(&Counterexample));
    assert!(
        matches!(&validated, PluginClaim::ValidatedUnsafe { violated } if violated.len() == 1),
        "{validated:?}"
    );
    assert_eq!(verdict(&validated), Verdict::Unsafe);
    let refuted = claim(Answer::Empty, None, Some(&Counterexample));
    assert!(
        matches!(refuted, PluginClaim::Failure { .. }),
        "{refuted:?}"
    );
    assert_eq!(verdict(&refuted), Verdict::Unknown);
}

#[test]
fn a_plugin_failure_or_an_unknown_answer_is_unknown() {
    for (answer, code) in [
        (Answer::Timeout, Some("QP006")),
        (Answer::Crash, Some("QP006")),
        (Answer::WrongRequest, Some("QP007")),
        (Answer::Unknown, None),
    ] {
        let claim = claim(answer, Some(&Replay), Some(&Counterexample));
        assert_eq!(claim.code(), code);
        assert_eq!(verdict(&claim), Verdict::Unknown, "{claim:?}");
    }
    // A gate: the request names a plugin nobody registered.
    let registry = VerifierRegistry::default();
    let task = task();
    let claim = registry.verify_plugin_claim(&request(&task), &task, Some(&Replay), None);
    assert_eq!(claim.code(), Some("QP002"));
    assert_eq!(verdict(&claim), Verdict::Unknown);
}

#[test]
fn the_classification_never_disagrees_with_the_registry_on_safe() {
    // `verify_v2` and the firewall read the same response through the same
    // checks: the firewall is `Safe` exactly when `verify_v2` certifies.
    for answer in [
        Answer::Certified,
        Answer::Partial,
        Answer::Bare,
        Answer::Misbound,
        Answer::Witnessed,
        Answer::Alleged,
        Answer::Unknown,
        Answer::WrongRequest,
    ] {
        let mut registry = VerifierRegistry::default();
        registry
            .register(Arc::new(Plugin {
                manifest: manifest(),
                answer,
            }))
            .unwrap();
        let task = task();
        let outcome = registry.verify_v2(&request(&task), &task, Some(&Replay));
        let claim = registry.verify_plugin_claim(&request(&task), &task, Some(&Replay), None);
        assert_eq!(
            outcome.certificate.is_some(),
            verdict(&claim) == Verdict::Safe
        );
        assert_ne!(verdict(&claim), Verdict::Unsafe);
    }
}
