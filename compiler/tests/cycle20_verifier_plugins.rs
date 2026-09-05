use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use qcore_compiler::{
    BuiltInProvider, CompileOptions, EvidenceClaim, FactPredicate, NormalizedFact,
    PLUGIN_EVIDENCE_VERSION, PLUGIN_PROTOCOL_VERSION, PluginEntrypoint, PluginId, ProcessProvider,
    ProposedEvidence, ProposedVerdict, ProviderFailure, Reproducibility, SourceFile, TargetProfile,
    UnitIdentity, VerificationRequest, VerificationResponse, VerifierCapability, VerifierProvider,
    VerifierRegistry, compile_files, compose_verification, decode_protocol_frame,
    encode_protocol_frame, first_party_manifests, first_party_registry, parse_source_program,
};

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

#[cfg(unix)]
#[test]
fn process_provider_timeout_is_unknown_and_cannot_issue_a_certificate() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let package = std::env::temp_dir().join(format!("qcore-plugin-timeout-{unique}"));
    fs::create_dir(&package).unwrap();
    let executable = package.join("hang.sh");
    fs::write(&executable, "#!/bin/sh\nwhile :; do :; done\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();

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
        .register(Arc::new(ProcessProvider::new(manifest, &package).unwrap()))
        .unwrap();
    let outcome = registry.verify(&request("example.hang", "libevent"));
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert!(outcome.certificate.is_none());
    assert_eq!(outcome.diagnostics[0].code, "QP006");

    fs::remove_dir_all(package).unwrap();
}

#[test]
fn package_discovery_loads_a_versioned_manifest() {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("qcore-plugin-discovery-{unique}"));
    let package = root.join("example-discovered");
    fs::create_dir_all(&package).unwrap();
    let mut manifest = first_party_manifests().remove(0);
    manifest.plugin_id = PluginId::new("example.discovered").unwrap();
    manifest.entrypoint = PluginEntrypoint::Executable {
        path: "provider".into(),
    };
    manifest.package_digest = digest('0');
    fs::write(
        package.join("verifier-plugin.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

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

    let module = compile_files(
        &[SourceFile::new("scope.qc", source)],
        &CompileOptions::default(),
    )
    .unwrap();
    assert_eq!(module.entry, "main");
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
    let manifest = first_party_manifests().remove(0);
    let req = request("qcore.libevent", "libevent");
    let response = echo_response(&manifest, &req);
    let evidence = response.evidence.unwrap();
    assert_eq!(
        evidence.reproducibility.provider_version,
        env!("CARGO_PKG_VERSION")
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
    let manifest = first_party_manifests().remove(0);
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
        os::unix::fs::PermissionsExt,
        time::{SystemTime, UNIX_EPOCH},
    };
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let package = std::env::temp_dir().join(format!("qcore-plugin-evidence-{unique}"));
    fs::create_dir(&package).unwrap();
    let executable = package.join("provider.sh");
    fs::write(
        &executable,
        "#!/bin/sh\n/bin/cat > /dev/null\nexec /bin/cat response.bin\n",
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let mut manifest = first_party_manifests().remove(0);
    manifest.plugin_id = PluginId::new("example.echo").unwrap();
    manifest.entrypoint = PluginEntrypoint::Executable {
        path: "provider.sh".into(),
    };
    manifest.package_digest = digest('0');
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(
            ProcessProvider::new(manifest.clone(), &package).unwrap(),
        ))
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
