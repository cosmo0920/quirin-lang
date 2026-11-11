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

#[cfg(unix)]
use qcore_compiler::UnverifiedProvider;
use qcore_compiler::{
    BuiltInProvider, CompileOptions, EvidenceClaim, FactPredicate, NormalizedFact,
    PLUGIN_EVIDENCE_VERSION, PLUGIN_PROTOCOL_VERSION, PluginEntrypoint, PluginId, ProposedEvidence,
    ProposedVerdict, ProviderFailure, Reproducibility, SourceFile, TargetProfile, UnitIdentity,
    VerificationRequest, VerificationResponse, VerifierCapability, VerifierProvider,
    VerifierRegistry, compile_files, compose_verification, decode_protocol_frame,
    encode_protocol_frame, first_party_manifests, first_party_registry, parse_source_program,
};

// Every executable fixture is written by a separate process, so a sibling
// test's concurrent spawn cannot make its `execve` fail with `ETXTBSY`
// (`tests/support/process_fixture.rs`). Data files stay `fs::write`.
#[path = "../../tests/support/process_fixture.rs"]
mod process_fixture;
#[cfg(unix)]
use process_fixture::write_executable;

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn request(plugin: &str, library: &str) -> VerificationRequest {
    VerificationRequest {
        protocol_version: PLUGIN_PROTOCOL_VERSION,
        evidence_version: PLUGIN_EVIDENCE_VERSION,
        request_id: "request-1".into(),
        plugin_id: PluginId::new(plugin).unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: library.into(),
            revision: "audited-revision".into(),
            profile: "default".into(),
            dialect: "c11".into(),
            platform: "linux".into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            source_digest: digest('a'),
            build_digest: digest('b'),
            translation_unit: "src/input.c".into(),
        },
        facts: vec![NormalizedFact {
            fact_id: "resource-1".into(),
            subject: "ctx".into(),
            predicate: FactPredicate::ResourceBalanced,
            holds: true,
        }],
        assumptions: BTreeMap::from([("callbacks_bounded".into(), true)]),
    }
}

#[test]
fn first_party_builtins_are_registered_as_separate_providers() {
    let manifests = first_party_manifests();
    assert_eq!(manifests.len(), 5);
    assert_eq!(
        manifests
            .iter()
            .map(|manifest| manifest.plugin_id.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "qcore.chunkio",
            "qcore.cmetrics",
            "qcore.ctraces",
            "qcore.libevent",
            "qcore.librdkafka",
        ])
    );
    assert!(manifests.iter().all(|manifest| manifest.validate().is_ok()));
}

#[test]
fn builtin_assertions_cannot_establish_safety() {
    let registry = first_party_registry();
    let outcome = registry.verify(&request("qcore.libevent", "libevent"));
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert!(outcome.certificate.is_none());
    assert!(
        outcome.diagnostics[0]
            .code
            .ends_with(".semantic_evidence_unavailable")
    );
}

#[test]
fn extracted_plugins_preserve_manifests_and_fail_closed_for_every_capability() {
    // P09 removed the per-plugin hardcoded `libevent::provider()` /
    // `chunkio::provider()` selection route after re-measured parity
    // (`docs/reports/remaining-p09/README.md`). The same expectations are now
    // asserted through the surviving profile-package discovery route; a
    // separately registered provider must still answer identically to the
    // discovered one.
    let first_party = first_party_registry();
    for plugin in ["qcore.libevent", "qcore.chunkio"] {
        let manifest = first_party_manifests()
            .into_iter()
            .find(|manifest| manifest.plugin_id.as_str() == plugin)
            .unwrap();
        let mut registry = VerifierRegistry::default();
        registry
            .register(Arc::new(BuiltInProvider::new(manifest.clone()).unwrap()))
            .unwrap();
        assert!(registry.manifests().contains(&&manifest));
        assert!(first_party.manifests().contains(&&manifest));
        for capability in &manifest.capabilities {
            let mut request = request(manifest.plugin_id.as_str(), &manifest.targets[0].library);
            request.capability = *capability;
            let outcome = registry.verify(&request);
            assert_eq!(outcome, first_party.verify(&request));
            assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
            assert!(outcome.certificate.is_none());
            assert_eq!(
                outcome.diagnostics[0].code,
                format!("{}.semantic_evidence_unavailable", manifest.plugin_id)
            );
        }
        // The deleted route forced a built-in adapter regardless of the
        // declared entrypoint. Nothing may reintroduce that: an executable
        // manifest is refused here, so only discovery's `ProcessProvider` arm
        // can serve one.
        let mut executable = manifest.clone();
        executable.entrypoint = PluginEntrypoint::Executable {
            path: "provider".into(),
        };
        executable.package_digest = digest('c');
        assert!(matches!(
            BuiltInProvider::new(executable),
            Err(qcore_compiler::ManifestError::InvalidTarget)
        ));
    }
}

// Deliberately untrusted provider: echo all caller assertions as a Safe proof,
// including the empty case. This must be rejected by the core, not the provider.
fn echo_response(
    manifest: &qcore_compiler::PluginManifest,
    request: &VerificationRequest,
) -> VerificationResponse {
    VerificationResponse {
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
                provider_version: manifest.package_version.clone(),
                package_digest: manifest.package_digest.clone(),
                deterministic: true,
            },
        }),
    }
}

struct EchoingProvider(BuiltInProvider);

impl VerifierProvider for EchoingProvider {
    fn manifest(&self) -> &qcore_compiler::PluginManifest {
        self.0.manifest()
    }
    fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, ProviderFailure> {
        Ok(echo_response(self.manifest(), request))
    }
}

#[derive(Clone)]
struct ForgingProvider {
    builtin: BuiltInProvider,
}

impl VerifierProvider for ForgingProvider {
    fn manifest(&self) -> &qcore_compiler::PluginManifest {
        self.builtin.manifest()
    }

    fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, ProviderFailure> {
        let mut response = echo_response(self.manifest(), request);
        let evidence = response.evidence.as_mut().unwrap();
        evidence.claims[0] = EvidenceClaim {
            fact_id: "invented".into(),
            subject: "ctx".into(),
            predicate: FactPredicate::ResourceBalanced,
            holds: true,
        };
        Ok(response)
    }
}

#[test]
fn forged_safe_evidence_is_downgraded_to_unknown() {
    let manifest = first_party_manifests()
        .into_iter()
        .find(|manifest| manifest.plugin_id.as_str() == "qcore.libevent")
        .unwrap();
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(ForgingProvider {
            builtin: BuiltInProvider::new(manifest).unwrap(),
        }))
        .unwrap();
    let outcome = registry.verify(&request("qcore.libevent", "libevent"));
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert!(outcome.certificate.is_none());
    assert_eq!(outcome.diagnostics[0].code, "QP009");
}

#[test]
fn missing_disabled_incompatible_and_negative_results_fail_closed() {
    let mut registry = first_party_registry();
    let missing = registry.verify(&request("example.missing", "libevent"));
    assert_eq!(missing.verdict, ProposedVerdict::Unknown);

    let id = PluginId::new("qcore.libevent").unwrap();
    registry.set_enabled(&id, false).unwrap();
    assert_eq!(
        registry
            .verify(&request("qcore.libevent", "libevent"))
            .verdict,
        ProposedVerdict::Unknown
    );
    registry.set_enabled(&id, true).unwrap();

    let mut incompatible = request("qcore.libevent", "libevent");
    incompatible.protocol_version += 1;
    assert_eq!(
        registry.verify(&incompatible).verdict,
        ProposedVerdict::Unknown
    );

    let mut unsafe_request = request("qcore.libevent", "libevent");
    unsafe_request.facts[0].holds = false;
    let unsafe_outcome = registry.verify(&unsafe_request);
    assert_eq!(unsafe_outcome.verdict, ProposedVerdict::Unknown);
    assert!(unsafe_outcome.certificate.is_none());
}

#[test]
fn composition_of_unsupported_assertions_stays_unknown() {
    let registry = first_party_registry();
    let first = registry.verify(&request("qcore.cmetrics", "cmetrics"));
    let mut second_request = request("qcore.ctraces", "ctraces");
    second_request
        .assumptions
        .insert("callbacks_bounded".into(), false);
    let second = registry.verify(&second_request);
    let composed = compose_verification(vec![first, second]);
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(composed.diagnostics[0].code, "QP011");
}

#[test]
fn composition_cannot_promote_an_uncertified_safe_label() {
    let forged = qcore_compiler::VerificationOutcome {
        verdict: ProposedVerdict::Safe,
        diagnostics: Vec::new(),
        certificate: None,
        composite: None,
    };
    let composed = compose_verification(vec![forged]);
    assert_eq!(composed.verdict, ProposedVerdict::Unknown);
    assert_eq!(composed.diagnostics[0].code, "QP014");
}

#[test]
fn protocol_frames_are_length_delimited_json_and_reject_trailing_data() {
    let request = request("qcore.libevent", "libevent");
    let frame = encode_protocol_frame(&request, 16 * 1024).unwrap();
    let decoded: VerificationRequest = decode_protocol_frame(&frame, 16 * 1024).unwrap();
    assert_eq!(decoded, request);
    let mut malformed = frame;
    malformed.push(0);
    assert!(decode_protocol_frame::<VerificationRequest>(&malformed, 16 * 1024).is_err());
}

#[test]
fn protocol_rejects_unknown_fields_instead_of_guessing_a_schema() {
    let request = request("qcore.libevent", "libevent");
    let mut value = serde_json::to_value(request).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("future_field".into(), serde_json::Value::Bool(true));
    let payload = serde_json::to_vec(&value).unwrap();
    let mut frame = Vec::from((payload.len() as u32).to_be_bytes());
    frame.extend(payload);
    assert!(decode_protocol_frame::<VerificationRequest>(&frame, 16 * 1024).is_err());
}

/// Installs a package that actually *verifies*: the script carries the manifest
/// template its installed manifest binds to, exactly as a shipped provider
/// executable does, after an `exit` that `sh` never reads past. A registry takes
/// only verified providers (card `P01-F3`), so a test that registers one must
/// install a real package rather than assert over an unchecked directory.
#[cfg(unix)]
fn install_verifiable(
    package: &std::path::Path,
    manifest: &qcore_compiler::PluginManifest,
) -> qcore_compiler::ProcessProvider {
    use qcore_compiler::verifier_plugin::{envelope, sha256_hex, verify_executable_package};
    let PluginEntrypoint::Executable { path } = &manifest.entrypoint else {
        panic!("an executable package");
    };
    let script = package.join(path);
    let mut bytes = std::fs::read(&script).unwrap();
    bytes.extend(b"exit $?\n");
    bytes.extend(envelope(&serde_json::to_string(manifest).unwrap()).into_bytes());
    bytes.push(b'\n');
    write_executable(&script, &bytes);
    let mut installed = manifest.clone();
    installed.package_digest = sha256_hex(&std::fs::read(&script).unwrap());
    std::fs::write(
        package.join("verifier-plugin.json"),
        serde_json::to_vec(&installed).unwrap(),
    )
    .unwrap();
    verify_executable_package(package).expect("the installed fixture verifies")
}

#[cfg(unix)]
#[test]
fn process_provider_timeout_is_unknown_and_cannot_issue_a_certificate() {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let package = std::env::temp_dir().join(format!("qcore-plugin-timeout-{unique}"));
    fs::create_dir(&package).unwrap();
    let executable = package.join("hang.sh");
    write_executable(&executable, b"#!/bin/sh\nwhile :; do :; done\n");

    let mut manifest = first_party_manifests()
        .into_iter()
        .find(|manifest| manifest.plugin_id.as_str() == "qcore.libevent")
        .unwrap();
    manifest.plugin_id = PluginId::new("example.hang").unwrap();
    manifest.entrypoint = PluginEntrypoint::Executable {
        path: "hang.sh".into(),
    };
    manifest.package_digest = digest('0');
    manifest.limits.timeout_ms = 20;
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(install_verifiable(&package, &manifest)))
        .unwrap();
    let outcome = registry.verify(&request("example.hang", "libevent"));
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert!(outcome.certificate.is_none());
    assert_eq!(outcome.diagnostics[0].code, "QP006");

    fs::remove_dir_all(package).unwrap();
}

/// Discovery registers a package only after **verifying** it (card `P01-F3`).
///
/// Before that card, discovery registered whatever carried a well-formed
/// `verifier-plugin.json` — including, as the first half here measures, a
/// package with no executable at all and a placeholder digest. Both halves run:
/// the manifest-only directory is now an error with nothing registered, and the
/// installed package is discovered.
#[cfg(unix)]
#[test]
fn package_discovery_verifies_before_it_registers() {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("qcore-plugin-discovery-{unique}"));
    let package = root.join("example-discovered");
    fs::create_dir_all(&package).unwrap();
    let mut manifest = first_party_manifests()
        .into_iter()
        .find(|manifest| manifest.plugin_id.as_str() == "qcore.libevent")
        .expect("Libevent fixture manifest");
    manifest.plugin_id = PluginId::new("example.discovered").unwrap();
    manifest.entrypoint = PluginEntrypoint::Executable {
        path: "provider".into(),
    };
    manifest.package_digest = digest('0');

    // The manifest alone, exactly as this test used to install it: no
    // executable, and a placeholder digest that names no file.
    fs::write(
        package.join("verifier-plugin.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let mut registry = VerifierRegistry::default();
    let errors = registry
        .discover_packages(&root)
        .expect_err("an unverifiable package is not registered");
    assert_eq!(errors.len(), 1);
    assert!(
        registry
            .manifest(&PluginId::new("example.discovered").unwrap())
            .is_none()
    );

    // The same directory, installed as a real package: the executable exists,
    // carries the manifest template, and the manifest carries its digest.
    write_executable(&package.join("provider"), b"#!/bin/sh\nexit 0\n");
    let provider = install_verifiable(&package, &manifest);
    assert_eq!(provider.manifest().plugin_id.as_str(), "example.discovered");
    let mut registry = VerifierRegistry::default();
    assert_eq!(registry.discover_packages(&root).unwrap(), 1);
    assert!(
        registry
            .manifest(&PluginId::new("example.discovered").unwrap())
            .is_some()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn verifier_scope_syntax_resolves_against_the_same_builtin_registry() {
    let source = r#"
verify scope fluent_bit {
  plugin qcore.libevent version "0.1" profile linux_epoll capabilities { event_traces, resource_contracts };
  plugin qcore.chunkio profile default capabilities { chunk_lifecycle };
}
fn main() -> unit { return; }
"#;
    let parsed = parse_source_program(source).unwrap();
    let scopes = first_party_registry()
        .resolve_source_scopes(&parsed)
        .unwrap();
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].name, "fluent_bit");
    assert_eq!(scopes[0].providers.len(), 2);

    let errors = compile_files(
        &[SourceFile::new("scope.qc", source)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "Q701");
}

#[test]
fn verifier_scope_rejects_unknown_capabilities_and_plugins() {
    let bad_capability = parse_source_program(
        "verify scope bad { plugin qcore.libevent profile default capabilities { omniscience }; } fn main() -> unit { return; }",
    )
    .unwrap();
    assert!(
        first_party_registry()
            .resolve_source_scopes(&bad_capability)
            .is_err()
    );

    let diagnostics = compile_files(
        &[SourceFile::new(
            "missing.qc",
            "verify scope bad { plugin example.missing profile default capabilities { resource_contracts }; } fn main() -> unit { return; }",
        )],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert_eq!(diagnostics[0].code, "Q700");
}

#[test]
fn reproducibility_metadata_does_not_establish_semantic_evidence() {
    let manifest = first_party_manifests()
        .into_iter()
        .find(|manifest| manifest.plugin_id.as_str() == "qcore.libevent")
        .expect("Libevent fixture manifest");
    let req = request("qcore.libevent", "libevent");
    let response = echo_response(&manifest, &req);
    let evidence = response.evidence.unwrap();
    // The provider version is the plugin *package* version the manifest
    // declares, not this crate's version. Those two strings were both `0.1.0`
    // until P09 versioned `qcore-compiler` to `0.2.0`, which exposed the
    // coincidence; the assertion is retargeted at the value the code actually
    // publishes, not relaxed.
    assert_eq!(
        evidence.reproducibility.provider_version,
        manifest.package_version
    );
    assert_eq!(
        evidence.reproducibility.package_digest,
        "builtin:qcore.libevent"
    );
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(EchoingProvider(
            BuiltInProvider::new(manifest).unwrap(),
        )))
        .unwrap();
    let outcome = registry.verify(&req);
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert!(outcome.certificate.is_none());
    assert_eq!(outcome.diagnostics[0].code, "QP009");
}

#[test]
fn vp01_predicate_source_profile_and_capability_mutations_cannot_mint_safety() {
    use FactPredicate::*;
    for manifest in first_party_manifests() {
        let builtin = first_party_registry();
        let mut custom = VerifierRegistry::default();
        custom
            .register(Arc::new(EchoingProvider(
                BuiltInProvider::new(manifest.clone()).unwrap(),
            )))
            .unwrap();
        // Vary each dimension independently and in combination, including the
        // matching capability predicate. No combination supplies a witness.
        for capability in &manifest.capabilities {
            for predicate in [
                ApiRecognized,
                IdentityUnique,
                ResourceBalanced,
                LifecycleTerminal,
                ConcurrencySafe,
                ObservationMatched,
                ProjectionComplete,
            ] {
                for source in ["src/input.c", "nonexistent.c"] {
                    for profile in ["default", "unaudited-profile"] {
                        for revision in ["audited-revision", "unaudited-revision"] {
                            let mut req =
                                request(manifest.plugin_id.as_str(), &manifest.targets[0].library);
                            req.capability = *capability;
                            req.facts[0].predicate = predicate;
                            req.unit.translation_unit = source.into();
                            req.target.profile = profile.into();
                            req.target.revision = revision.into();
                            for registry in [&builtin, &custom] {
                                let outcome = registry.verify(&req);
                                assert_eq!(outcome.verdict, ProposedVerdict::Unknown, "{req:?}");
                                assert!(outcome.certificate.is_none());
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn vp02_empty_in_process_evidence_is_rejected_by_the_core() {
    let manifest = first_party_manifests()
        .into_iter()
        .find(|manifest| manifest.plugin_id.as_str() == "qcore.libevent")
        .expect("Libevent fixture manifest");
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(EchoingProvider(
            BuiltInProvider::new(manifest).unwrap(),
        )))
        .unwrap();
    let mut req = request("qcore.libevent", "libevent");
    req.facts.clear();
    let outcome = registry.verify(&req);
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert!(outcome.certificate.is_none());
    assert_eq!(outcome.diagnostics[0].code, "QP009");
    assert!(outcome.diagnostics[0].message.contains("no-obligations"));
}

#[cfg(unix)]
#[test]
fn vp01_vp02_process_proposals_reach_the_firewall_and_cannot_mint_safety() {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let package = std::env::temp_dir().join(format!("qcore-plugin-evidence-{unique}"));
    fs::create_dir(&package).unwrap();
    let executable = package.join("provider.sh");
    write_executable(
        &executable,
        b"#!/bin/sh\n/bin/cat > /dev/null\nexec /bin/cat response.bin\n",
    );
    let mut manifest = first_party_manifests()
        .into_iter()
        .find(|manifest| manifest.plugin_id.as_str() == "qcore.libevent")
        .expect("Libevent fixture manifest");
    manifest.plugin_id = PluginId::new("example.echo").unwrap();
    manifest.entrypoint = PluginEntrypoint::Executable {
        path: "provider.sh".into(),
    };
    manifest.package_digest = digest('0');
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(install_verifiable(&package, &manifest)))
        .unwrap();
    for empty in [false, true] {
        let mut req = request("example.echo", "libevent");
        if empty {
            req.facts.clear();
        }
        let response = echo_response(&manifest, &req);
        fs::write(
            package.join("response.bin"),
            encode_protocol_frame(&response, 16 * 1024).unwrap(),
        )
        .unwrap();
        let outcome = registry.verify(&req);
        assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
        assert!(outcome.certificate.is_none());
        // QP009 proves the executable ran and returned a valid envelope;
        // rejection must not merely be a transport or process failure.
        assert_eq!(outcome.diagnostics[0].code, "QP009", "{outcome:?}");
        if empty {
            assert!(outcome.diagnostics[0].message.contains("no-obligations"));
        }
    }
    fs::remove_dir_all(package).unwrap();
}

#[test]
fn vp06_unverified_unsafe_is_unknown_and_cannot_dominate_composition() {
    struct Allegation(PluginManifest);
    use qcore_compiler::PluginManifest;
    impl VerifierProvider for Allegation {
        fn manifest(&self) -> &PluginManifest {
            &self.0
        }
        fn verify(
            &self,
            req: &VerificationRequest,
        ) -> Result<VerificationResponse, ProviderFailure> {
            Ok(VerificationResponse {
                protocol_version: req.protocol_version,
                request_id: req.request_id.clone(),
                plugin_id: req.plugin_id.clone(),
                verdict: ProposedVerdict::Unsafe,
                diagnostics: vec![],
                evidence: None,
            })
        }
    }
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(Allegation(
            first_party_manifests()
                .into_iter()
                .find(|manifest| manifest.plugin_id.as_str() == "qcore.libevent")
                .expect("Libevent fixture manifest"),
        )))
        .unwrap();
    let outcome = registry.verify(&request("qcore.libevent", "libevent"));
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert_eq!(outcome.diagnostics[0].code, "QP010");
    assert_eq!(
        compose_verification(vec![outcome]).verdict,
        ProposedVerdict::Unknown
    );
}

#[test]
fn vp05_nonempty_source_obligations_cannot_silently_skip_providers() {
    for plugin in ["qcore.libevent", "example.unavailable"] {
        let source = format!(
            "verify scope check {{ plugin {plugin} profile default capabilities {{ resource_contracts }}; }} fn main() -> i64 {{ return 7; }}"
        );
        let errors = compile_files(
            &[SourceFile::new("scope.qc", source)],
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert_eq!(
            errors[0].code,
            if plugin == "qcore.libevent" {
                "Q701"
            } else {
                "Q700"
            }
        );
    }
}

#[cfg(unix)]
#[test]
fn vp04_vp07_vp08_process_lifecycle_and_payload_budgets() {
    use std::{
        fs,
        sync::mpsc,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let package = std::env::temp_dir().join(format!(
        "qcore-process-regression-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&package).unwrap();
    let script = package.join("provider.sh");
    let mut manifest = first_party_manifests()
        .into_iter()
        .find(|manifest| manifest.plugin_id.as_str() == "qcore.libevent")
        .expect("Libevent fixture manifest");
    manifest.entrypoint = PluginEntrypoint::Executable {
        path: "provider.sh".into(),
    };
    manifest.limits.timeout_ms = 20;
    manifest.package_digest = digest('0');
    let run = |manifest: qcore_compiler::PluginManifest, req: VerificationRequest| {
        let provider = UnverifiedProvider::new(manifest, &package).unwrap();
        let (send, recv) = mpsc::channel();
        let start = Instant::now();
        std::thread::spawn(move || {
            let _ = send.send(provider.verify(&req));
        });
        // Outer harness bound catches regressions in transport cleanup.
        let result = recv
            .recv_timeout(Duration::from_secs(2))
            .expect("provider exceeded outer harness deadline");
        (result, start.elapsed())
    };
    let write_script = |body: &str| {
        write_executable(&script, format!("#!/bin/sh\n{body}\n").as_bytes());
    };
    // Both waiting and already-exited direct children can leave inherited pipes.
    for tail in ["wait", "exit 0"] {
        write_script(&format!(
            "(/bin/sleep 0.15; : > descendant.marker) &\n{tail}"
        ));
        let (result, elapsed) = run(manifest.clone(), request("qcore.libevent", "libevent"));
        assert_eq!(result, Err(ProviderFailure::Timeout));
        assert!(elapsed < Duration::from_millis(200), "{elapsed:?}");
        std::thread::sleep(Duration::from_millis(200));
        assert!(!package.join("descendant.marker").exists());
    }
    write_script(": > launched.marker");
    let mut rejected = manifest.clone();
    rejected.limits.output_bytes = 1;
    assert_eq!(
        run(rejected, request("qcore.libevent", "libevent")).0,
        Err(ProviderFailure::OutputLimit)
    );
    std::thread::sleep(Duration::from_millis(30));
    assert!(!package.join("launched.marker").exists());

    // A closed input pipe must clean up after spawn, even with unread input.
    // Give the newly spawned shell enough time to run the close on slower
    // macOS schedulers. The provider must remain alive beyond that allowance
    // so this still distinguishes a broken input pipe from a timeout.
    write_script("exec 0<&-\n/bin/sleep 0.3\n: > io.marker");
    let mut large = request("qcore.libevent", "libevent");
    large.request_id = "x".repeat(8 * 1024 * 1024);
    let mut closed_input = manifest.clone();
    closed_input.limits.timeout_ms = 100;
    closed_input.limits.output_bytes = 16 * 1024 * 1024;
    let (result, elapsed) = run(closed_input, large);
    // `Io` before card HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING; still not
    // a timeout on Linux. Other Unix hosts keep a read end as their
    // consumption probe, so the pipe cannot break and this is a timeout.
    #[cfg(target_os = "linux")]
    assert!(
        matches!(result, Err(ProviderFailure::RequestUnconsumed(_))),
        "{result:?}"
    );
    #[cfg(not(target_os = "linux"))]
    assert_eq!(result, Err(ProviderFailure::Timeout));
    assert!(elapsed < Duration::from_millis(500));
    std::thread::sleep(Duration::from_millis(350));
    assert!(!package.join("io.marker").exists());

    let req = request("qcore.libevent", "libevent");
    let reply = VerificationResponse {
        protocol_version: req.protocol_version,
        request_id: req.request_id.clone(),
        plugin_id: req.plugin_id.clone(),
        verdict: ProposedVerdict::Unknown,
        evidence: None,
        diagnostics: vec![qcore_compiler::PluginDiagnostic {
            code: "qcore.libevent.pending".into(),
            message: "x".repeat(4096),
            fact_id: None,
        }],
    };
    let payload = serde_json::to_vec(&reply).unwrap().len();
    let frame = encode_protocol_frame(&reply, payload).unwrap();
    fs::write(package.join("response.bin"), &frame).unwrap();
    manifest.limits.output_bytes = payload;
    manifest.limits.timeout_ms = 1000;
    write_script("/bin/cat > /dev/null\n/bin/sleep 0.03\n/bin/cat response.bin");
    assert_eq!(run(manifest.clone(), req.clone()).0.unwrap(), reply);
    manifest.limits.output_bytes = payload - 1;
    assert_eq!(
        run(manifest.clone(), req.clone()).0,
        Err(ProviderFailure::OutputLimit)
    );
    manifest.limits.output_bytes = payload;
    // Successful completion still terminates descendants with closed pipes.
    write_script(
        "/bin/cat > /dev/null\n(/bin/sleep 0.15; : > success.marker) </dev/null >/dev/null 2>/dev/null &\n/bin/cat response.bin",
    );
    assert!(run(manifest, req).0.is_ok());
    std::thread::sleep(Duration::from_millis(200));
    assert!(!package.join("success.marker").exists());
    fs::remove_dir_all(package).unwrap();
}
