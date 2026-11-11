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

//! These tests build the host without the compiler, audit driver or any
//! first-party package. They pin the extracted trust boundary independently.
use qcore_verifier_host::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

// Only `write_executable` is used here (`support/process_fixture.rs`).
#[cfg(unix)]
#[allow(dead_code)]
#[path = "support/process_fixture.rs"]
mod process_fixture;

fn request() -> VerificationRequest {
    VerificationRequest {
        protocol_version: 1,
        evidence_version: 1,
        request_id: "request".into(),
        plugin_id: PluginId::new("example.echo").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "example".into(),
            revision: "source".into(),
            profile: "default".into(),
            dialect: "qcore".into(),
            platform: "generic".into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
            translation_unit: "unit.qcore".into(),
        },
        facts: vec![],
        assumptions: BTreeMap::new(),
    }
}
fn manifest() -> PluginManifest {
    PluginManifest {
        plugin_id: PluginId::new("example.echo").unwrap(),
        package_version: "0.1.0".into(),
        protocol_versions: BTreeSet::from([1]),
        evidence_versions: BTreeSet::from([1]),
        entrypoint: PluginEntrypoint::BuiltIn,
        package_digest: "builtin:example.echo".into(),
        targets: vec![SupportedTarget {
            library: "example".into(),
            revisions: BTreeSet::from(["source".into()]),
            profiles: BTreeSet::from(["default".into()]),
            dialects: BTreeSet::from(["qcore".into()]),
            platforms: BTreeSet::from(["generic".into()]),
            backends: BTreeSet::from(["native".into()]),
        }],
        capabilities: BTreeSet::from([VerifierCapability::ResourceContracts]),
        required_foreign_summaries: BTreeSet::new(),
        required_observation_hooks: BTreeSet::new(),
        filesystem_inputs: BTreeSet::new(),
        network_access: false,
        limits: PluginLimits::default(),
        trust_class: PluginTrustClass::FirstParty,
        certificate_checker: "example.untrusted_claims".into(),
        lean_theorem_family: "QCore.VerifierPlugin".into(),
    }
}
struct Echo(PluginManifest);
impl VerifierProvider for Echo {
    fn manifest(&self) -> &PluginManifest {
        &self.0
    }
    fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, ProviderFailure> {
        Ok(VerificationResponse {
            protocol_version: request.protocol_version,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict: ProposedVerdict::Safe,
            diagnostics: vec![],
            evidence: Some(ProposedEvidence {
                evidence_version: request.evidence_version,
                unit: request.unit.clone(),
                capability: request.capability,
                claims: request
                    .facts
                    .iter()
                    .map(|fact| EvidenceClaim {
                        fact_id: fact.fact_id.clone(),
                        subject: fact.subject.clone(),
                        predicate: fact.predicate,
                        holds: fact.holds,
                    })
                    .collect(),
                coverage: request
                    .facts
                    .iter()
                    .map(|fact| fact.fact_id.clone())
                    .collect(),
                assumptions_used: request.assumptions.clone(),
                reproducibility: Reproducibility {
                    provider_version: self.0.package_version.clone(),
                    package_digest: self.0.package_digest.clone(),
                    deterministic: true,
                },
            }),
        })
    }
}

#[test]
fn zero_provider_registry_is_independent_and_fail_closed() {
    let registry = VerifierRegistry::default();
    assert!(registry.manifests().is_empty());
    let outcome = registry.verify(&request());
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert_eq!(outcome.diagnostics[0].code, "QP002");
    assert!(outcome.certificate.is_none());
    let program = qcore_frontend::frontend::parse_source_program("verify scope check { plugin example.echo profile default capabilities { resource_contracts }; } fn main() -> unit { return; }").unwrap();
    assert!(registry.resolve_source_scopes(&program).is_err());
}

#[test]
fn standalone_host_refuses_v1_assertions_with_empty_and_nonempty_coverage() {
    let mut registry = VerifierRegistry::default();
    registry.register(Arc::new(Echo(manifest()))).unwrap();
    for facts in [
        vec![],
        vec![NormalizedFact {
            fact_id: "claim".into(),
            subject: "resource".into(),
            predicate: FactPredicate::ResourceBalanced,
            holds: true,
        }],
    ] {
        let mut request = request();
        request.facts = facts;
        let outcome = registry.verify(&request);
        assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
        assert!(outcome.certificate.is_none());
        assert_eq!(outcome.diagnostics[0].code, "QP009");
    }
}

#[cfg(unix)]
#[test]
fn closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request() {
    use std::{
        fs,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    let package = std::env::temp_dir().join(format!(
        "qcore-host-closed-input-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&package).unwrap();
    let script = package.join("provider.sh");
    let mut manifest = manifest();
    manifest.entrypoint = PluginEntrypoint::Executable {
        path: "provider.sh".into(),
    };
    manifest.package_digest = "a".repeat(64);
    manifest.limits.timeout_ms = 40;
    let mut request = request();
    // Larger than Linux's and macOS's maximum dynamically grown pipe capacity:
    // closing input necessarily interrupts transmission, even if the parent
    // gets the first scheduler time slice.
    request.request_id = "x".repeat(8 * 1024 * 1024);
    manifest.limits.output_bytes = 16 * 1024 * 1024;
    let write_script = |body: &str| {
        process_fixture::write_executable(&script, format!("#!/bin/sh\n{body}\n").as_bytes());
    };
    write_script("exec 0<&-\n(/bin/sleep 0.3; : > descendant.marker) &\nexit 0");
    let provider = UnverifiedProvider::new(manifest.clone(), &package).unwrap();
    let start = Instant::now();
    assert_eq!(provider.verify(&request), Err(ProviderFailure::Timeout));
    assert!(start.elapsed() < Duration::from_secs(1));
    std::thread::sleep(Duration::from_millis(350));
    assert!(!package.join("descendant.marker").exists());

    write_script("exec 0<&-\n/bin/sleep 0.3\n: > live.marker");
    // Allow the shell to execute the close even on a slower macOS scheduler;
    // it stays alive beyond this deadline, so Timeout would still be wrong.
    manifest.limits.timeout_ms = 100;
    let provider = UnverifiedProvider::new(manifest.clone(), &package).unwrap();
    let result = provider.verify(&request);
    // On Linux the host holds no read end while it writes, so the closed input
    // breaks the pipe and the refusal names the unconsumed request (card
    // `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`; `Io` before it). Other
    // Unix hosts keep a read end from the spawn as their consumption probe, so
    // the pipe cannot break there and a provider alive at its deadline is a
    // timeout. Both refuse.
    #[cfg(target_os = "linux")]
    assert!(
        matches!(result, Err(ProviderFailure::RequestUnconsumed(_))),
        "{result:?}"
    );
    #[cfg(not(target_os = "linux"))]
    assert_eq!(result, Err(ProviderFailure::Timeout));
    std::thread::sleep(Duration::from_millis(350));
    assert!(!package.join("live.marker").exists());

    // The oversized request forces a broken pipe even when the host writes
    // first. Once all pipes close, a failed exit must take precedence over
    // that transport symptom, including stderr detail and signal exits.
    manifest.limits.timeout_ms = 1000;
    for (body, code, signal) in [
        ("exec 0<&-\nexit 6", Some(6), None),
        ("exec 0<&-\necho denied >&2\nexit 6", Some(6), None),
        ("exec 0<&-\nkill -TERM $$", None, Some(15)),
    ] {
        write_script(body);
        let provider = UnverifiedProvider::new(manifest.clone(), &package).unwrap();
        let failure = provider.verify(&request).unwrap_err();
        let record = provider.last_execution().unwrap();
        assert_eq!(record.outcome, ProcessOutcome::Terminated, "{failure}");
        assert_eq!(record.exit_code, code);
        assert_eq!(record.signal, signal);
        if body.contains("denied") {
            assert!(failure.to_string().contains("denied"), "{failure}");
        }
    }

    // Even a valid response cannot erase the host's incomplete-request state.
    let response = VerificationResponse {
        protocol_version: request.protocol_version,
        request_id: request.request_id.clone(),
        plugin_id: request.plugin_id.clone(),
        verdict: ProposedVerdict::Unknown,
        evidence: None,
        diagnostics: vec![],
    };
    fs::write(
        package.join("response.bin"),
        encode_protocol_frame(&response, manifest.limits.output_bytes).unwrap(),
    )
    .unwrap();
    write_script("exec 0<&-\n/bin/cat response.bin");
    manifest.limits.timeout_ms = 1000;
    let provider = UnverifiedProvider::new(manifest.clone(), &package).unwrap();
    let result = provider.verify(&request);
    assert!(
        matches!(result, Err(ProviderFailure::RequestUnconsumed(_))),
        "{result:?}"
    );
    assert_eq!(
        provider.last_execution().unwrap().outcome,
        ProcessOutcome::RequestUnconsumed
    );
    // The same large response succeeds when the provider consumes the whole
    // request. This also pins active-pipe progress within the unchanged budget.
    write_script("/bin/cat > /dev/null\n/bin/cat response.bin");
    let provider = UnverifiedProvider::new(manifest, &package).unwrap();
    assert_eq!(provider.verify(&request), Ok(response));
    fs::remove_dir_all(package).unwrap();
}
