//! Real process-host acceptance; this harness is not a provider dependency.
#![cfg(unix)]
use qcore_schemas::plugin::*;
use qcore_schemas::{obligation::*, sha256_hex};
use qcore_verifier_host::{
    BuiltInProvider, ProcessProvider, VerifierProvider, VerifierRegistry, encode_protocol_frame,
};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
fn request(manifest: &PluginManifest, capability: VerifierCapability) -> VerificationRequest {
    VerificationRequest {
        protocol_version: 1,
        evidence_version: 1,
        request_id: "conformance".into(),
        plugin_id: manifest.plugin_id.clone(),
        capability,
        target: TargetProfile {
            library: manifest.targets[0].library.clone(),
            revision: "pinned".into(),
            profile: "default".into(),
            dialect: "c11".into(),
            platform: "linux".into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            translation_unit: "fixture.c".into(),
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
        },
        facts: vec![NormalizedFact {
            fact_id: "claimed".into(),
            subject: "function".into(),
            predicate: FactPredicate::ResourceBalanced,
            holds: true,
        }],
        assumptions: BTreeMap::new(),
    }
}
fn v2(request: &VerificationRequest) -> (VerificationRequestV2, VerificationTask) {
    let property = match request.capability {
        VerifierCapability::ResourceContracts => ObligationProperty::CustodyDischarged,
        VerifierCapability::EventTraces => ObligationProperty::EventGenerationDischarged,
        VerifierCapability::ChunkLifecycle => ObligationProperty::ChunkDischarged,
        VerifierCapability::Concurrency => ObligationProperty::JoinBeforeExit,
        VerifierCapability::TranslationUnitProjection => ObligationProperty::ProjectionAccounted,
        VerifierCapability::ObservationHooks => ObligationProperty::ObservationChecked,
    };
    let task = VerificationTask {
        task_version: 1,
        unit: request.unit.clone(),
        target: request.target.clone(),
        semantic_profile: "process-conformance".into(),
        contract_sources: BTreeMap::new(),
        bounds: TaskBounds::single_path(),
        assumptions: BTreeMap::new(),
        evidence_version: 2,
        obligations: vec![Obligation {
            obligation_id: "core-obligation".into(),
            capability: request.capability,
            property,
            subject: ObligationSubject::Unit,
            scope: ObligationScope::WholeUnit,
            bounds: TaskBounds::single_path(),
            assumptions: BTreeMap::new(),
            evidence_version: 2,
        }],
        derivation_incomplete: vec![],
    };
    (
        VerificationRequestV2 {
            source_effect: None,
            counter_effect: None,
            protocol_version: 1,
            evidence_version: 2,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            capability: request.capability,
            target: request.target.clone(),
            unit: request.unit.clone(),
            facts: request.facts.clone(),
            assumptions: request.assumptions.clone(),
            task_digest: task.task_digest(),
            obligations: task.obligations.clone(),
        },
        task,
    )
}
fn raw(package: &Path, frame: &[u8]) -> Output {
    let mut child = Command::new(package.join("provider"))
        .current_dir(package)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(frame).unwrap();
    child.wait_with_output().unwrap()
}
fn json_frame(value: &serde_json::Value) -> Vec<u8> {
    encode_protocol_frame(value, DEFAULT_MAX_FRAME_BYTES).unwrap()
}
fn rejected(package: &Path, wire: &[u8]) {
    let output = raw(package, wire);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty(), "invalid input emitted a response");
    assert!(!output.stderr.is_empty());
}

#[test]
fn real_packages_preserve_unknown_and_reject_mutations() {
    let root = root();
    let output =
        std::env::temp_dir().join(format!("qcore-provider-conformance-{}", std::process::id()));
    let built = Command::new("python3")
        .arg(root.join("package.py"))
        .args(["build", "--output"])
        .arg(&output)
        .arg("--target-dir")
        .arg(root.join("target/conformance-artifacts"))
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("packages.json")).unwrap()).unwrap();
    assert_eq!(index["artifacts"].as_array().unwrap().len(), 5);
    let mut cases = 0;
    let mut parity = Vec::new();
    for artifact in index["artifacts"].as_array().unwrap() {
        let id = artifact["plugin_id"].as_str().unwrap();
        let package = output.join(id);
        let process = ProcessProvider::from_package(&package).unwrap();
        let manifest = process.manifest().clone();
        let original_path = root
            .parent()
            .unwrap()
            .join("contract/profiles")
            .join(if id == "qcore.libevent" {
                "libevent"
            } else {
                "fluent-bit"
            })
            .join("providers")
            .join(format!("{id}.json"));
        let baseline: PluginManifest =
            serde_json::from_slice(&std::fs::read(original_path).unwrap()).unwrap();
        let mut normalized = manifest.clone();
        normalized.entrypoint = baseline.entrypoint.clone();
        normalized.package_digest = baseline.package_digest.clone();
        normalized.evidence_versions = baseline.evidence_versions.clone();
        let mut expected = baseline.clone();
        if id == "qcore.cmetrics" {
            expected.package_version = "0.2.0".into();
            expected.certificate_checker = "qcore.boolean_field_contract.v2".into();
            expected.lean_theorem_family = "QCore.ChunkSource".into();
            expected.targets[0].dialects = ["c11", "gnu99", "gnu11", "gnu17", "qcore"]
                .into_iter()
                .map(str::to_owned)
                .collect();
        }
        if id == "qcore.chunkio" {
            // Explicit additive source-contract package revision. Existing v1
            // capabilities keep their Unknown behavior below; no built-in route
            // or source-set profile is silently updated by package installation.
            // Deliberate pin update: package 0.5.0 adds the counter-contract
            // capability route. The certificate checker, Lean family and every
            // existing target/profile entry are unchanged.
            expected.package_version = "0.5.0".into();
            expected.certificate_checker = "qcore.boolean_field_contract.v2".into();
            expected.lean_theorem_family = "QCore.ChunkSource".into();
            expected.targets[0].dialects = ["c11", "gnu99", "gnu11", "gnu17", "qcore"]
                .into_iter()
                .map(str::to_owned)
                .collect();
            expected.targets.push(SupportedTarget {
                library: "fluent_bit".into(),
                revisions: ["*".into()].into(),
                profiles: [
                    "qcore.c_boolean_field_contract.v2".into(),
                    "qcore.c_boolean_field_contract.v3".into(),
                    "qcore.c_counter_contract.v1".into(),
                ]
                .into(),
                dialects: ["c11", "gnu99", "gnu11", "gnu17"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                platforms: ["*".into()].into(),
                backends: ["*".into()].into(),
            });
        }
        assert_eq!(normalized, expected, "declared routing metadata for {id}");
        let builtin = BuiltInProvider::new(baseline.clone()).unwrap();
        let mut registry = VerifierRegistry::default();
        registry
            .register(Arc::new(ProcessProvider::from_package(&package).unwrap()))
            .unwrap();
        let mut builtin_registry = VerifierRegistry::default();
        builtin_registry
            .register(Arc::new(BuiltInProvider::new(baseline).unwrap()))
            .unwrap();
        for capability in &manifest.capabilities {
            let mut request = request(&manifest, *capability);
            for fact_state in [None, Some(false), Some(true)] {
                request.facts = fact_state
                    .map(|holds| NormalizedFact {
                        fact_id: "claimed".into(),
                        subject: "function".into(),
                        predicate: FactPredicate::ResourceBalanced,
                        holds,
                    })
                    .into_iter()
                    .collect();
                let response = process.verify(&request).unwrap();
                assert_eq!(response, builtin.verify(&request).unwrap());
                let outcome = registry.verify(&request);
                assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
                assert!(outcome.certificate.is_none());
                assert_eq!(
                    outcome.diagnostics,
                    builtin_registry.verify(&request).diagnostics
                );
                cases += 1;
            }
            let (request, task) = v2(&request);
            let response = process.verify_v2(&request).unwrap();
            assert_eq!(response.verdict, ProposedVerdict::Unknown);
            assert!(response.evidence.is_none());
            let outcome = registry.verify_v2(&request, &task, None);
            assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
            assert!(outcome.certificate.is_none());
            assert_eq!(outcome.diagnostics, response.diagnostics);
            parity.push(serde_json::json!({
                "plugin_id": manifest.plugin_id.as_str(), "capability": capability,
                "executable_sha256": manifest.package_digest,
                "v1_exact_response_parity_cases": 3,
                "v1_verdict": "unknown", "v1_certificate": false,
                "v2_response": response, "v2_certificate": outcome.certificate.is_some()
            }));
        }
        let request = request(&manifest, *manifest.capabilities.first().unwrap());
        let original = serde_json::to_value(&request).unwrap();
        for (pointer, value) in [
            ("/plugin_id", serde_json::json!("example.wrong")),
            ("/request_id", serde_json::json!("")),
            ("/protocol_version", serde_json::json!(2)),
            ("/evidence_version", serde_json::json!(3)),
            ("/target/library", serde_json::json!("wrong")),
            ("/target/profile", serde_json::json!("")),
            ("/target/profile", serde_json::json!("*")),
            ("/target/dialect", serde_json::json!("fortran")),
            ("/unit/source_digest", serde_json::json!("bad")),
        ] {
            let mut changed = original.clone();
            *changed.pointer_mut(pointer).unwrap() = value;
            rejected(&package, &json_frame(&changed));
        }
        let mut unsupported = original.clone();
        let capability = [
            VerifierCapability::ResourceContracts,
            VerifierCapability::EventTraces,
            VerifierCapability::ChunkLifecycle,
            VerifierCapability::Concurrency,
            VerifierCapability::TranslationUnitProjection,
            VerifierCapability::ObservationHooks,
        ]
        .into_iter()
        .find(|capability| !manifest.capabilities.contains(capability))
        .unwrap();
        unsupported["capability"] = serde_json::to_value(capability).unwrap();
        rejected(&package, &json_frame(&unsupported));
        let (request_v2, _) = v2(&request);
        for pointer in ["/task_digest", "/obligations/0/obligation_id"] {
            let mut changed = serde_json::to_value(&request_v2).unwrap();
            *changed.pointer_mut(pointer).unwrap() = serde_json::json!("");
            rejected(&package, &json_frame(&changed));
        }
        let mut unknown_field = original.clone();
        unknown_field["evidence"] = serde_json::json!({});
        rejected(&package, &json_frame(&unknown_field));
        let valid = json_frame(&original);
        for invalid in [
            vec![],
            vec![0, 0],
            vec![0, 0, 0, 0],
            vec![255; 4],
            valid[..valid.len() - 1].to_vec(),
            [valid.clone(), vec![0]].concat(),
            [valid.clone(), valid.clone()].concat(),
            vec![0, 0, 0, 1, b'{'],
        ] {
            rejected(&package, &invalid);
        }
        let manifest_path = package.join("verifier-plugin.json");
        let manifest_bytes = std::fs::read(&manifest_path).unwrap();
        for mutate in 0..4 {
            let mut changed = manifest.clone();
            match mutate {
                0 => changed.package_version = "999.0.0".into(),
                1 => changed.targets[0].profiles = ["other".into()].into(),
                2 => changed.package_digest = "f".repeat(64),
                _ => changed.evidence_versions = [1].into(),
            }
            std::fs::write(&manifest_path, serde_json::to_vec(&changed).unwrap()).unwrap();
            rejected(&package, &valid);
            let provider = ProcessProvider::from_package(&package).unwrap();
            assert!(provider.verify(&request).is_err());
        }
        std::fs::write(&manifest_path, &manifest_bytes).unwrap();
        let executable_path = package.join("provider");
        let original_executable = std::fs::read(&executable_path).unwrap();
        let mut changed_executable = original_executable.clone();
        changed_executable.push(0); // ELF still runs, so exercise its self-digest check.
        std::fs::write(&executable_path, changed_executable).unwrap();
        let output = raw(&package, &valid);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        if cfg!(target_os = "linux") {
            assert!(String::from_utf8_lossy(&output.stderr).contains("installed manifest differs"));
        }
        std::fs::write(&executable_path, original_executable).unwrap();
        assert_eq!(
            sha256_hex(&std::fs::read(package.join("provider")).unwrap()),
            manifest.package_digest
        );
    }
    println!(
        "five process packages: {cases} exact v1 response/Unknown parity cases; all declared capabilities return v2 Unknown; identity/version/framing/package mutations rejected"
    );
    println!("PARITY_JSON {}", serde_json::to_string(&parity).unwrap());
    std::fs::remove_dir_all(output).unwrap();
}

#[test]
fn artifact_dependencies_are_only_schemas_and_serialization() {
    let metadata = Command::new("cargo")
        .args([
            "metadata",
            "--offline",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(root().join("Cargo.toml"))
        .output()
        .unwrap();
    assert!(metadata.status.success());
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout).unwrap();
    let packages = metadata["packages"].as_array().unwrap();
    let mut artifacts = 0;
    for package in packages {
        let name = package["name"].as_str().unwrap();
        if name.starts_with("qcore-provider-") && name != "qcore-provider-conformance" {
            artifacts += 1;
            for dependency in package["dependencies"].as_array().unwrap() {
                assert!(
                    matches!(
                        dependency["name"].as_str().unwrap(),
                        "qcore-schemas" | "serde" | "serde_json"
                    ),
                    "{name}: {dependency}"
                );
                assert!(
                    dependency["kind"].is_null(),
                    "provider may not smuggle host/build dependencies"
                );
            }
        }
        if package["name"] == "qcore-schemas" {
            assert!(
                package["dependencies"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|dep| dep["path"].is_null())
            );
        }
        // The conformance harness may exercise the real host, but only as a test
        // consumer: its normal dependencies (the ones a schemas-only process
        // fixture in `src/bin` links) stay on the admitted schema surface.
        if package["name"] == "qcore-provider-conformance" {
            let mut host_dev_dependencies = 0;
            for dependency in package["dependencies"].as_array().unwrap() {
                let name = dependency["name"].as_str().unwrap();
                let dev = dependency["kind"].as_str() == Some("dev");
                if dev {
                    host_dev_dependencies += 1;
                    continue;
                }
                assert!(
                    matches!(name, "qcore-schemas" | "serde" | "serde_json"),
                    "conformance fixtures may not link `{name}` outside dev-dependencies"
                );
            }
            assert!(
                host_dev_dependencies > 0,
                "the harness is expected to consume the real host as a dev dependency"
            );
        }
    }
    assert_eq!(artifacts, 5);
}

/// The packaged artifacts are exactly the crates carrying a package template;
/// the conformance harness (which does link host crates for tests) is never one
/// of them, so no host crate can reach an installed provider package.
#[test]
fn only_template_carrying_crates_are_packaged() {
    let root = root();
    let mut templates: Vec<String> = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            path.join("provider-template.json")
                .is_file()
                .then(|| path.file_name().unwrap().to_string_lossy().into_owned())
        })
        .collect();
    templates.sort();
    assert_eq!(
        templates,
        [
            "qcore-provider-chunkio",
            "qcore-provider-cmetrics",
            "qcore-provider-ctraces",
            "qcore-provider-libevent",
            "qcore-provider-librdkafka",
        ]
    );
    assert!(!root.join("conformance/provider-template.json").exists());
}

/// Every shipped package answers identically under the target-qualified
/// enforced execution profile, and the run records what was enforced.
///
/// This is card P02's "well-behaved provider under generous limits still
/// works" case, run against real binaries rather than a shell fixture. The
/// profile is selected explicitly; nothing here changes the default profile the
/// other tests exercise.
#[test]
#[cfg(target_os = "linux")]
fn real_packages_answer_identically_under_the_enforced_profile() {
    use qcore_verifier_host::{
        LimitSource, ProcessExecutionProfile, ProcessOutcome, enforceable_limit_kinds,
    };
    let root = root();
    let output = std::env::temp_dir().join(format!(
        "qcore-provider-enforced-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let built = Command::new("python3")
        .arg(root.join("package.py"))
        .args(["build", "--output"])
        .arg(&output)
        .arg("--target-dir")
        .arg(root.join("target/conformance-artifacts"))
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("packages.json")).unwrap()).unwrap();
    let mut checked = 0;
    for artifact in index["artifacts"].as_array().unwrap() {
        let package = output.join(artifact["plugin_id"].as_str().unwrap());
        let legacy = ProcessProvider::from_package(&package).unwrap();
        let strict = ProcessProvider::from_package(&package)
            .unwrap()
            .with_execution_profile(ProcessExecutionProfile::LinuxRlimitV1);
        let manifest = strict.manifest().clone();
        // No shipped manifest declares a kind this profile cannot enforce, so
        // none of them is refused.
        let enforced = strict.enforced_limits().unwrap();
        assert!(enforced.unenforced_kinds.is_empty());
        assert!(enforced.process_group_kill);
        assert_eq!(
            enforced
                .rlimits
                .iter()
                .map(|limit| limit.kind.as_str())
                .collect::<Vec<_>>(),
            ["core_bytes", "cpu_seconds", "memory_bytes"]
        );
        for limit in &enforced.rlimits {
            assert!(
                enforceable_limit_kinds(ProcessExecutionProfile::LinuxRlimitV1)
                    .contains(&limit.kind.as_str())
            );
        }
        assert_eq!(
            enforced
                .rlimits
                .iter()
                .find(|limit| limit.kind == "memory_bytes")
                .map(|limit| (limit.soft, limit.source)),
            Some((manifest.limits.memory_bytes, LimitSource::Declared))
        );
        for capability in &manifest.capabilities {
            let request = request(&manifest, *capability);
            assert_eq!(
                strict.verify(&request).unwrap(),
                legacy.verify(&request).unwrap()
            );
            let record = strict.last_execution().unwrap();
            assert_eq!(record.outcome, ProcessOutcome::Completed);
            assert_eq!(record.exit_code, Some(0));
            assert_eq!(record.profile, ProcessExecutionProfile::LinuxRlimitV1);
            assert!(record.summary().contains("linux-rlimit-v1"));
            let (v2_request, task) = v2(&request);
            assert_eq!(
                strict.verify_v2(&v2_request).unwrap(),
                legacy.verify_v2(&v2_request).unwrap()
            );
            let mut registry = VerifierRegistry::default();
            registry
                .register(Arc::new(
                    ProcessProvider::from_package(&package)
                        .unwrap()
                        .with_execution_profile(ProcessExecutionProfile::LinuxRlimitV1),
                ))
                .unwrap();
            let outcome = registry.verify_v2(&v2_request, &task, None);
            assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
            assert!(outcome.certificate.is_none());
            checked += 1;
        }
    }
    // Five packages, eleven declared capabilities between them.
    assert_eq!(index["artifacts"].as_array().unwrap().len(), 5);
    assert_eq!(checked, 11);
    std::fs::remove_dir_all(&output).unwrap();
}
