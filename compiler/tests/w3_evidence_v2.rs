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

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

// The built provider is copied into its package by a separate process
// (`tests/support/process_fixture.rs`): only one test here spawns, so no
// sibling forks while it is written; the shared writer keeps it that way if
// another spawning test is added.
#[cfg(unix)]
#[path = "../../tests/support/process_fixture.rs"]
mod process_fixture;

use qcore_compiler::obligation_derivation::{DerivationInputs, DerivedTask, derive_task};
use qcore_compiler::verifier_plugin::*;
use qcore_compiler::witness_replay::ContractObservationReplay;
use qcore_compiler::{
    CompileOptions, SourceFile, compile_files_with_registry, parse_source_program,
};

const SOURCE: &str = "extern fn resize() -> unit contract { requires size > 0; ensures normal live == 1; };\nverify scope observed { plugin example.conformance profile default capabilities { observation_hooks }; }\nfn main() -> i32 { return 7; }";

fn manifest() -> PluginManifest {
    let mut manifest = first_party_manifests().remove(0);
    manifest.plugin_id = PluginId::new("example.conformance").unwrap();
    manifest.targets = vec![SupportedTarget {
        library: "conformance".into(),
        revisions: BTreeSet::from(["source".into()]),
        profiles: BTreeSet::from(["default".into()]),
        dialects: BTreeSet::from(["qcore".into()]),
        platforms: BTreeSet::from([std::env::consts::OS.into()]),
        backends: BTreeSet::from(["native".into()]),
    }];
    manifest.capabilities = BTreeSet::from([VerifierCapability::ObservationHooks]);
    manifest.evidence_versions = BTreeSet::from([1, 2]);
    manifest.required_observation_hooks = BTreeSet::from(["resize".into()]);
    manifest.certificate_checker = "qcore.contract_observation.v2".into();
    manifest.entrypoint = PluginEntrypoint::Executable {
        path: "provider".into(),
    };
    manifest.package_digest = "c".repeat(64);
    manifest
}

fn task() -> DerivedTask {
    task_with_assumptions(BTreeMap::new())
}

fn task_with_assumptions(assumptions: BTreeMap<String, bool>) -> DerivedTask {
    let parsed = parse_source_program(SOURCE).unwrap();
    derive_task(
        &DerivationInputs {
            translation_unit: "unit.qc",
            program: &parsed,
            target: TargetProfile {
                library: "conformance".into(),
                revision: "source".into(),
                profile: "default".into(),
                dialect: "qcore".into(),
                platform: std::env::consts::OS.into(),
                backend: "native".into(),
            },
            contract_sources: BTreeMap::from([("unit.qc".into(), SOURCE.into())]),
            required_observation_hooks: BTreeSet::from(["resize".into()]),
            assumptions,
            // The dispatcher derives for a named provider (E03-B3), so the
            // helper must too or the pinned request bytes will not match.
            verifier: Some("example.conformance".to_owned()),
        },
        VerifierCapability::ObservationHooks,
    )
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
        protocol_version: 1,
        evidence_version: 2,
        request_id: format!(
            "example.conformance:observation_hooks:{}",
            &task.task_digest()[..16]
        ),
        plugin_id: manifest().plugin_id,
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

fn response(request: &VerificationRequestV2) -> VerificationResponseV2 {
    VerificationResponseV2 {
        protocol_version: 1,
        request_id: request.request_id.clone(),
        plugin_id: request.plugin_id.clone(),
        verdict: ProposedVerdict::Safe,
        diagnostics: vec![],
        evidence: Some(ProposedEvidenceV2 {
            evidence_version: 2,
            unit: request.unit.clone(),
            capability: request.capability,
            coverage: request
                .obligations
                .iter()
                .map(|o| o.obligation_id.clone())
                .collect(),
            assumptions_used: request.assumptions.clone(),
            reproducibility: Reproducibility {
                provider_version: manifest().package_version,
                package_digest: manifest().package_digest,
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
                        seed_or_input: "size=4".into(),
                        profile: "default".into(),
                        instrumentation_version: "fixture-v1".into(),
                    },
                    observations: vec![ContractObservationRecord {
                        function: "resize".into(),
                        hook: "resize".into(),
                        values: BTreeMap::from([("size".into(), 4), ("live".into(), 1)]),
                    }],
                }],
            }),
        }),
    }
}

struct Provider {
    manifest: PluginManifest,
    mutation: fn(&mut VerificationResponseV2),
}
impl VerifierProvider for Provider {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    fn verify(&self, _: &VerificationRequest) -> Result<VerificationResponse, ProviderFailure> {
        Err(ProviderFailure::Protocol("fixture expects v2".into()))
    }
    fn verify_v2(
        &self,
        req: &VerificationRequestV2,
    ) -> Result<VerificationResponseV2, ProviderFailure> {
        let mut result = response(req);
        (self.mutation)(&mut result);
        Ok(result)
    }
}
fn registry(mutation: fn(&mut VerificationResponseV2)) -> VerifierRegistry {
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(Provider {
            manifest: manifest(),
            mutation,
        }))
        .unwrap();
    registry
}
fn observations(response: &mut VerificationResponseV2) -> &mut ContractObservationWitness {
    let SemanticWitness::ContractObservation(witness) =
        &mut response.evidence.as_mut().unwrap().witness
    else {
        panic!("fixture kind")
    };
    witness
}

#[test]
fn complete_scope_dispatch_replays_observations_and_retains_claim_bounds() {
    let registry = registry(|_| {});
    let parsed = parse_source_program(SOURCE).unwrap();
    let resolved = registry.resolve_source_scopes(&parsed).unwrap();
    let report =
        qcore_compiler::verify_dispatch::dispatch_scopes(&parsed, "unit.qc", &resolved, &registry);
    assert!(report.scopes[0].verified(), "{:?}", report.diagnostics());
    assert!(report.scopes[0].capabilities[0].dispatched);
    let certificate = report.scopes[0].composed.certificate.as_ref().unwrap();
    assert!(certificate.task().unwrap().bounds.observed_runs_only);
    assert_eq!(
        certificate
            .task()
            .unwrap()
            .assumptions
            .get("observations_trusted"),
        Some(&true)
    );
    assert_eq!(certificate.coverage().len(), 2);
    assert!(
        compile_files_with_registry(
            &[SourceFile::new("unit.qc", SOURCE)],
            &CompileOptions::default(),
            &registry
        )
        .is_ok()
    );
}

#[test]
fn malicious_v2_proposals_cannot_change_the_denominator_or_replay() {
    let derived = task();
    let request = request(&derived.task);
    let checker = ContractObservationReplay::new(derived.replay.observation.as_ref().unwrap());
    let mutations: &[fn(&mut VerificationResponseV2)] = &[
        |r| r.evidence = None,
        |r| r.evidence.as_mut().unwrap().coverage.clear(),
        |r| {
            r.evidence
                .as_mut()
                .unwrap()
                .coverage
                .insert("phantom".into());
        },
        |r| r.evidence.as_mut().unwrap().assumptions_used.clear(),
        |r| r.evidence.as_mut().unwrap().unit.source_digest = "d".repeat(64),
        |r| observations(r).task_digest = "d".repeat(64),
        |r| observations(r).runs.clear(),
        |r| observations(r).runs[0].observations.clear(),
        |r| {
            observations(r).runs[0].observations[0]
                .values
                .insert("size".into(), 0);
        },
        |r| {
            observations(r).runs[0].observations[0]
                .values
                .insert("live".into(), 0);
        },
        |r| {
            observations(r).runs[0].observations[0]
                .values
                .remove("live");
        },
        |r| observations(r).runs[0].binding.profile = "other".into(),
        |r| {
            let run = observations(r).runs[0].clone();
            observations(r).runs.push(run);
        },
        |r| r.verdict = ProposedVerdict::Unsafe,
    ];
    for (index, mutation) in mutations.iter().enumerate() {
        let result = registry(*mutation).verify_v2(&request, &derived.task, Some(&checker));
        assert_eq!(result.verdict, ProposedVerdict::Unknown, "mutation {index}");
        assert!(result.certificate.is_none());
    }
    assert_eq!(
        registry(|_| {})
            .verify_v2(&request, &derived.task, None)
            .verdict,
        ProposedVerdict::Unknown
    );
}

#[test]
fn task_envelope_cannot_be_rebound_and_incomplete_derivations_stay_unknown() {
    let mut derived = task();
    let valid = request(&derived.task);
    let checker = ContractObservationReplay::new(derived.replay.observation.as_ref().unwrap());
    let registry = registry(|_| {});
    for field in 0..4 {
        let mut request = valid.clone();
        match field {
            0 => request.unit.source_digest = "d".repeat(64),
            1 => request.target.revision = "other".into(),
            2 => request.assumptions.clear(),
            _ => request.obligations.clear(),
        }
        assert_eq!(
            registry
                .verify_v2(&request, &derived.task, Some(&checker))
                .verdict,
            ProposedVerdict::Unknown
        );
    }
    derived
        .task
        .derivation_incomplete
        .push(DerivationIncomplete {
            capability: VerifierCapability::ObservationHooks,
            category: qcore_schemas::ReasonCategory::IncompleteObligationCoverage,
            reason: "missing importer".into(),
        });
    assert_eq!(
        registry
            .verify_v2(&request(&derived.task), &derived.task, Some(&checker))
            .verdict,
        ProposedVerdict::Unknown
    );
}

#[cfg(unix)]
#[test]
fn out_of_process_v2_conformance_replays_and_rejects_a_forged_observation() {
    use std::{
        fs,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };
    let package = std::env::temp_dir().join(format!(
        "qcore-w3-process-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&package).unwrap();
    let fixture_manifest = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/w3/provider/Cargo.toml"
    );
    let build = Command::new(env!("CARGO"))
        .args([
            "build",
            "--offline",
            "--locked",
            "--manifest-path",
            fixture_manifest,
            "--target-dir",
        ])
        .arg(package.join("target"))
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "standalone provider must build against schemas alone: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    process_fixture::copy_executable(
        &package.join("target/debug/qcore-v2-conformance"),
        &package.join("provider"),
        0o755,
    );
    let derived = task();
    let request = request(&derived.task);
    fs::write(
        package.join("request.bin"),
        encode_protocol_frame(&request, DEFAULT_MAX_FRAME_BYTES).unwrap(),
    )
    .unwrap();
    let mut manifest = manifest();
    manifest.entrypoint = PluginEntrypoint::Executable {
        path: "provider".into(),
    };
    fs::write(
        package.join("verifier-plugin.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let mut registry = VerifierRegistry::default();
    // This fixture is a standalone, schemas-only executable built by the test;
    // it carries no embedded manifest template, so it cannot verify (card
    // `P01-F3`), and `UnverifiedProvider` is deliberately not a
    // `VerifierProvider`. The property under test is evidence-v2 replay and
    // forgery rejection, not package identity, so the wrapper below is written
    // out by hand — which is exactly the explicit act the type demands, and
    // which nothing in production performs.
    struct DeliberatelyUnverified(UnverifiedProvider);
    impl VerifierProvider for DeliberatelyUnverified {
        fn manifest(&self) -> &PluginManifest {
            self.0.manifest()
        }
        fn verify(
            &self,
            request: &VerificationRequest,
        ) -> Result<VerificationResponse, ProviderFailure> {
            self.0.verify(request)
        }
        fn verify_v2(
            &self,
            request: &VerificationRequestV2,
        ) -> Result<VerificationResponseV2, ProviderFailure> {
            self.0.verify_v2(request)
        }
    }
    registry
        .register(Arc::new(DeliberatelyUnverified(
            UnverifiedProvider::load(&package).unwrap(),
        )))
        .unwrap();
    let parsed = parse_source_program(SOURCE).unwrap();
    let scopes = registry.resolve_source_scopes(&parsed).unwrap();
    let report =
        qcore_compiler::verify_dispatch::dispatch_scopes(&parsed, "unit.qc", &scopes, &registry);
    assert!(package.join("invoked").exists());
    assert!(report.scopes[0].verified(), "{:?}", report.diagnostics());
    fs::write(package.join("response-mode"), "bad_postcondition").unwrap();
    let report =
        qcore_compiler::verify_dispatch::dispatch_scopes(&parsed, "unit.qc", &scopes, &registry);
    assert!(!report.scopes[0].verified());
    assert!(report.scopes[0].composed.certificate.is_none());
    fs::remove_dir_all(package).unwrap();
}

#[test]
fn composition_requires_exact_nonempty_inventory() {
    let derived = task();
    let checker = ContractObservationReplay::new(derived.replay.observation.as_ref().unwrap());
    let outcome =
        registry(|_| {}).verify_v2(&request(&derived.task), &derived.task, Some(&checker));
    let required = derived
        .task
        .coverage_for(VerifierCapability::ObservationHooks);
    assert_eq!(
        compose_verification_with_inventory(vec![outcome.clone()], &required).verdict,
        ProposedVerdict::Safe
    );
    let mut missing = required.clone();
    missing.insert("additional-required-obligation".into());
    assert_eq!(
        compose_verification_with_inventory(vec![outcome.clone()], &missing).verdict,
        ProposedVerdict::Unknown
    );
    let mut subset = required.clone();
    subset.pop_first();
    assert_eq!(
        compose_verification_with_inventory(vec![outcome.clone()], &subset).verdict,
        ProposedVerdict::Unknown
    );
    assert_eq!(
        compose_verification_with_inventory(vec![outcome], &BTreeSet::new()).verdict,
        ProposedVerdict::Unknown
    );
    assert_eq!(
        compose_verification_with_inventory(vec![], &required).verdict,
        ProposedVerdict::Unknown
    );
}

#[test]
fn explicit_observation_distrust_cannot_dispatch_or_mint_a_certificate() {
    let derived = task_with_assumptions(BTreeMap::from([("observations_trusted".into(), false)]));
    assert_eq!(
        derived.task.assumptions.get("observations_trusted"),
        Some(&false)
    );
    let checker = ContractObservationReplay::new(derived.replay.observation.as_ref().unwrap());
    let outcome = registry(|_| panic!("explicit distrust must fail before provider dispatch"))
        .verify_v2(&request(&derived.task), &derived.task, Some(&checker));
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert!(outcome.certificate.is_none());
    assert_eq!(outcome.diagnostics[0].code, "QP016");
}

#[test]
fn replay_requires_declared_trust_and_observed_run_bounds() {
    for mutation in 0..5 {
        let mut derived = task();
        match mutation {
            0 => {
                derived.task.assumptions.remove("observations_trusted");
            }
            1 => {
                derived
                    .task
                    .assumptions
                    .insert("observations_trusted".into(), false);
            }
            2 => derived.task.bounds.observed_runs_only = false,
            3 => derived.task.obligations[0].bounds.observed_runs_only = false,
            _ => {
                derived.task.obligations[0]
                    .assumptions
                    .insert("observations_trusted".into(), false);
            }
        }
        let request = request(&derived.task);
        let checker = ContractObservationReplay::new(derived.replay.observation.as_ref().unwrap());
        let witness = response(&request).evidence.unwrap().witness;
        assert!(
            checker.check(&derived.task, &witness).is_err(),
            "direct replay mutation {mutation}"
        );
        let outcome = registry(|_| {}).verify_v2(&request, &derived.task, Some(&checker));
        assert_eq!(
            outcome.verdict,
            ProposedVerdict::Unknown,
            "firewall mutation {mutation}"
        );
        assert!(outcome.certificate.is_none());
    }
}

#[test]
fn observation_replay_rejects_relabelled_scope_module_and_unchecked_bounds() {
    use qcore_schemas::obligation::{ObligationScope, ObligationSubject, PathBounds};
    let mut accepted = Vec::new();
    for mutation in 0..7 {
        let mut derived = task();
        match mutation {
            0 => {
                derived.task.obligations[0].scope = ObligationScope::Functions {
                    symbols: BTreeSet::from(["unobserved_function".into()]),
                }
            }
            1 => {
                derived.task.obligations[0].scope = ObligationScope::ObservedRuns {
                    run_ids: BTreeSet::from(["unobserved_run".into()]),
                }
            }
            2 => {
                let ObligationSubject::ObservedContract {
                    contract_module, ..
                } = &mut derived
                    .task
                    .obligations
                    .iter_mut()
                    .find(|o| matches!(o.subject, ObligationSubject::ObservedContract { .. }))
                    .unwrap()
                    .subject
                else {
                    panic!("fixture subject")
                };
                *contract_module = "unobserved_contract.qc".into();
            }
            3 => derived.task.bounds.schedules_explored = Some(100),
            4 => {
                derived.task.obligations[0].bounds.path_exploration =
                    PathBounds::BoundedPaths { paths: 100 }
            }
            5 => derived.task.obligations[0].bounds.max_operations = Some(0),
            _ => {
                derived.task.obligations[1].scope = ObligationScope::Functions {
                    symbols: BTreeSet::from(["unobserved_function".into()]),
                }
            }
        }
        let request = request(&derived.task);
        let checker = ContractObservationReplay::new(derived.replay.observation.as_ref().unwrap());
        let witness = response(&request).evidence.unwrap().witness;
        if checker.check(&derived.task, &witness).is_ok() {
            accepted.push((mutation, "direct"));
        }
        let outcome = registry(|_| {}).verify_v2(&request, &derived.task, Some(&checker));
        if outcome.verdict != ProposedVerdict::Unknown || outcome.certificate.is_some() {
            accepted.push((mutation, "firewall"));
        }
    }
    assert!(
        accepted.is_empty(),
        "scope/module/bounds substitutions accepted: {accepted:?}"
    );
}
