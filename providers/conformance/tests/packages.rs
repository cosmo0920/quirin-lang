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

//! Real process-host acceptance; this harness is not a provider dependency.
#![cfg(unix)]
use qcore_schemas::plugin::*;
use qcore_schemas::{obligation::*, sha256_hex};
use qcore_verifier_host::{
    BuiltInProvider, ProcessProvider, UnverifiedProvider, VerifierProvider, VerifierRegistry,
    encode_protocol_frame,
};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;

#[path = "../../../tests/support/tracked_files.rs"]
mod tracked_files;

// A package executable this binary rewrites and then runs is written by a
// separate process, so a sibling test's concurrent spawn cannot make its
// `execve` fail with `ETXTBSY` (`tests/support/process_fixture.rs`). Its mode
// is kept. Copies that are only read, and data files, stay `fs::write`.
#[path = "../../../tests/support/process_fixture.rs"]
mod process_fixture;

/// The permission bits of `path`, so a rewrite keeps them.
fn mode_of(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).unwrap().permissions().mode() & 0o7777
}

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
            reactor_program: None,
            teardown_source: None,
            call_use_source: None,
            cleanup_replay_source: None,
            composition_source: None,
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
/// P01-F2: a shipped package's declarations must be bound to the template its
/// own executable carries, by reading bytes only. Each edit below is
/// schema-valid and leaves `package_digest` intact, so before this binding
/// existed the read-only CLI verifier accepted all of them; the fields are
/// exactly those an enforced execution profile installs plus the routing scope.
fn embedded_template_binding(shipped: &Path, manifest: &PluginManifest) -> usize {
    let executable = std::fs::read(shipped.join("provider")).unwrap();
    let count = |needle: &str| {
        executable
            .windows(needle.len())
            .filter(|window| *window == needle.as_bytes())
            .count()
    };
    assert_eq!(count(qcore_verifier_host::TEMPLATE_BEGIN), 1);
    assert_eq!(count(qcore_verifier_host::TEMPLATE_END), 1);
    let template = qcore_verifier_host::extract_template(&executable).unwrap();
    assert_eq!(template.plugin_id, manifest.plugin_id);
    qcore_verifier_host::verify_executable_package(shipped).expect("shipped package verifies");
    // Tamper on a copy: the shipped package stays byte-identical, and no file
    // another thread may execute is ever rewritten here.
    let package = &shipped.with_file_name(format!(
        "{}.binding",
        shipped.file_name().unwrap().to_string_lossy()
    ));
    std::fs::create_dir_all(package).unwrap();
    std::fs::write(package.join("provider"), &executable).unwrap();
    let manifest_path = package.join("verifier-plugin.json");
    let original = std::fs::read(shipped.join("verifier-plugin.json")).unwrap();
    std::fs::write(&manifest_path, &original).unwrap();
    qcore_verifier_host::verify_executable_package(package).expect("the copy verifies too");
    type Edit = (&'static str, Box<dyn Fn(&mut PluginManifest)>);
    let edits: Vec<Edit> = vec![
        (
            "limits.timeout_ms",
            Box::new(|m: &mut PluginManifest| m.limits.timeout_ms *= 12),
        ),
        (
            "limits.memory_bytes",
            Box::new(|m: &mut PluginManifest| m.limits.memory_bytes *= 4),
        ),
        (
            "limits.output_bytes",
            Box::new(|m: &mut PluginManifest| m.limits.output_bytes *= 2),
        ),
        (
            "limits.additional.cpu_seconds",
            Box::new(|m: &mut PluginManifest| {
                m.limits.additional.insert("cpu_seconds".into(), 3_600);
            }),
        ),
        (
            "limits.additional.max_processes",
            Box::new(|m: &mut PluginManifest| {
                m.limits.additional.insert("max_processes".into(), 4_096);
            }),
        ),
        (
            "targets[0].library",
            Box::new(|m: &mut PluginManifest| m.targets[0].library = "other_library".into()),
        ),
        (
            "targets[0].platforms",
            Box::new(|m: &mut PluginManifest| m.targets[0].platforms = ["*".into()].into()),
        ),
        (
            "capabilities",
            Box::new(|m: &mut PluginManifest| {
                m.capabilities.insert(VerifierCapability::Concurrency);
            }),
        ),
        (
            "filesystem_inputs",
            Box::new(|m: &mut PluginManifest| m.filesystem_inputs = ["extra".into()].into()),
        ),
        (
            "trust_class",
            Box::new(|m: &mut PluginManifest| m.trust_class = PluginTrustClass::ThirdParty),
        ),
    ];
    let mut refused = 0;
    for (field, edit) in &edits {
        let mut changed = manifest.clone();
        edit(&mut changed);
        if changed == *manifest {
            continue; // this package already declares the edited value
        }
        assert!(changed.validate().is_ok(), "{field} stays schema-valid");
        std::fs::write(&manifest_path, serde_json::to_vec_pretty(&changed).unwrap()).unwrap();
        let Err(error) = qcore_verifier_host::verify_executable_package(package) else {
            panic!("digest-intact declaration edit must be refused: {field}");
        };
        assert!(
            error
                .message
                .contains(&format!("installed manifest field `{field}` diverges")),
            "{field}: {}",
            error.message
        );
        refused += 1;
    }
    // A stripped envelope refuses distinctly instead of downgrading. Both
    // markers are overwritten in place, so the executable keeps its length and
    // only the envelope disappears.
    let mut stripped = executable.clone();
    for marker in [
        qcore_verifier_host::TEMPLATE_BEGIN,
        qcore_verifier_host::TEMPLATE_END,
    ] {
        let start = stripped
            .windows(marker.len())
            .position(|window| window == marker.as_bytes())
            .unwrap();
        stripped.splice(
            start..start + marker.len(),
            std::iter::repeat_n(b'.', marker.len()),
        );
    }
    let mut without = manifest.clone();
    without.package_digest = sha256_hex(&stripped);
    std::fs::write(&manifest_path, serde_json::to_vec_pretty(&without).unwrap()).unwrap();
    std::fs::write(package.join("provider"), &stripped).unwrap();
    let Err(error) = qcore_verifier_host::verify_executable_package(package) else {
        panic!("a stripped envelope must be refused");
    };
    assert!(error.message.contains("carries no"), "{}", error.message);
    std::fs::remove_dir_all(package).unwrap();
    // The shipped package was never written to, and still verifies.
    assert_eq!(std::fs::read(shipped.join("provider")).unwrap(), executable);
    qcore_verifier_host::verify_executable_package(shipped)
        .expect("shipped package still verifies");
    refused + 1
}

fn rejected(package: &Path, wire: &[u8]) {
    let output = raw(package, wire);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty(), "invalid input emitted a response");
    assert!(!output.stderr.is_empty());
}

/// Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING: a real package whose contracts
/// `#include` libc modules pins every reached parent in its inventory and
/// installs the pinned text under `includes/`, `package.py verify` accepts it,
/// and verify refuses a **changed** and a **missing** included parent. Each
/// mutation is undone by rewriting the original bytes, the round trip is
/// asserted, and the package verifies again. Returns the number of refused
/// mutations (0 for a package that includes nothing).
fn included_parent_mutations(root: &Path, package: &Path) -> usize {
    let inventory: serde_json::Value =
        serde_json::from_slice(&std::fs::read(package.join("contracts.json")).unwrap()).unwrap();
    let Some(pins) = inventory.get("includes").and_then(|pins| pins.as_object()) else {
        assert!(!package.join("includes").exists());
        return 0;
    };
    let verify = || {
        Command::new("python3")
            .arg(root.join("package.py"))
            .arg("verify")
            .arg(package)
            .output()
            .unwrap()
    };
    let refused = |expected: &str, what: &str| {
        let output = verify();
        assert!(!output.status.success(), "{what} must not verify");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected), "{what}: {stderr}");
    };
    assert!(verify().status.success());
    let mut controls = 0;
    for (key, pin) in pins {
        let path = package
            .join("includes")
            .join(pin["crate"].as_str().unwrap())
            .join(pin["module"].as_str().unwrap());
        let original = std::fs::read(&path).unwrap();
        assert_eq!(
            sha256_hex(&original),
            pin["sha256"].as_str().unwrap(),
            "{key}"
        );
        let mut changed = original.clone();
        changed.extend_from_slice(b"// changed\n");
        std::fs::write(&path, &changed).unwrap();
        refused("included parent changed", key);
        std::fs::remove_file(&path).unwrap();
        refused("included parent set mismatch", key);
        std::fs::write(&path, &original).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), original, "{key}: round trip");
        assert!(
            verify().status.success(),
            "{key}: restored package verifies"
        );
        controls += 2;
    }
    controls
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
    let mut bound = 0;
    let mut including: Vec<String> = Vec::new();
    let mut include_controls = 0;
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
        if id == "qcore.libevent" {
            // Deliberate pin update: package 0.2.0 adds the reactor callback
            // capability route, so the executable no longer only proposes
            // unsupported assertions and names the checker that actually
            // admits its witness. Targets, capabilities, limits, trust class
            // and the Lean family are unchanged, and the built-in v1 adapter
            // below still answers Unknown for every capability.
            expected.package_version = "0.2.0".into();
            expected.certificate_checker = "qcore.reactor_callback_program.v1".into();
        }
        if id == "qcore.chunkio" {
            // Explicit additive source-contract package revision. Existing v1
            // capabilities keep their Unknown behavior below; no built-in route
            // or source-set profile is silently updated by package installation.
            // Deliberate pin update: package 0.5.0 added the counter-contract
            // capability route and 0.6.0 adds the two teardown-contract routes
            // (list unlink, nullable-field destructor). Both new profiles are
            // reached through the `chunkio` target, whose profile list is
            // already `*`, so the certificate checker, Lean family and every
            // existing target/profile entry are unchanged. 0.7.0 adds the
            // call-result def/use route on the same `chunkio` target, under
            // the same already-`*` profile list, 0.8.0 adds the symbolic
            // cleanup-replay route on that same target and profile list, and
            // 0.9.0 adds the unmap-wrapper composition route on it too (card
            // P04-COMPOSE), and 0.10.0 the two `cio_file_close` composition
            // routes, whose own prerequisite is that composition (card
            // P04-COMPOSE-A). All of them are reached through the same
            // `chunkio` target, whose profile list is already `*`, so no other
            // manifest field moves.
            expected.package_version = "0.13.0".into();
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
            // The host refuses the mutated package outright (card `P01-F3`);
            // the executable's own self-check is a *separate* property, so it
            // is driven through the loader that says "unverified" in its type
            // and that no registry accepts.
            assert!(
                qcore_verifier_host::verify_executable_package(&package).is_err(),
                "a mutated manifest must not verify"
            );
            let provider = UnverifiedProvider::load(&package).unwrap();
            assert!(provider.verify(&request).is_err());
        }
        std::fs::write(&manifest_path, &manifest_bytes).unwrap();
        let executable_path = package.join("provider");
        let original_executable = std::fs::read(&executable_path).unwrap();
        let mut changed_executable = original_executable.clone();
        changed_executable.push(0); // ELF still runs, so exercise its self-digest check.
        let mode = mode_of(&executable_path);
        process_fixture::write_executable_mode(&executable_path, &changed_executable, mode);
        let output = raw(&package, &valid);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        if cfg!(target_os = "linux") {
            assert!(String::from_utf8_lossy(&output.stderr).contains("installed manifest differs"));
        }
        process_fixture::write_executable_mode(&executable_path, &original_executable, mode);
        assert_eq!(
            sha256_hex(&std::fs::read(package.join("provider")).unwrap()),
            manifest.package_digest
        );
        bound += embedded_template_binding(&package, &manifest);
        let controls = included_parent_mutations(&root, &package);
        if controls > 0 {
            including.push(id.to_owned());
            include_controls += controls;
        }
    }
    // Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING: exactly the two packages
    // whose contracts inherit libc contracts carry include pins.
    assert_eq!(including, ["qcore.libevent", "qcore.librdkafka"]);
    println!(
        "five process packages: {cases} exact v1 response/Unknown parity cases; all declared capabilities return v2 Unknown; identity/version/framing/package mutations rejected; {bound} digest-intact declaration edits and stripped envelopes refused by the embedded-template binding; {include_controls} changed or missing included parents refused in {} packages",
        including.len()
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
    let mut shared = 0;
    let mut sdk = 0;
    let mut data_edges: Vec<(String, String)> = Vec::new();
    let mut artifact_contract_edges: Vec<(String, String)> = Vec::new();
    for package in packages {
        let name = package["name"].as_str().unwrap();
        // An artifact's only admitted first-party dependency is the shared
        // provider library, and every `quirin-provider-*` library is itself held
        // to the same schemas-and-serialization surface (plus its sibling shared
        // libraries), so the artifact's whole dependency closure stays on it.
        let is_artifact =
            name.starts_with("qcore-provider-") && name != "qcore-provider-conformance";
        // Card W45-USER-CONTRACT-PLUGINS (deliberate pin update): the SDK for
        // user contract crates and checker plugins carries the data-crate
        // prefix but owns no contracts; it links the shared provider transport
        // for its checker runtime and nothing else first-party.
        let is_sdk = name == "quirin-provider-plugin";
        if is_sdk {
            sdk += 1;
            for dependency in package["dependencies"].as_array().unwrap() {
                let dependency_name = dependency["name"].as_str().unwrap();
                assert!(
                    matches!(
                        dependency_name,
                        "qcore-schemas" | "serde" | "serde_json" | "quirin-provider-shared"
                    ) && dependency["kind"].is_null(),
                    "{name}: {dependency}"
                );
            }
            continue;
        }
        let is_shared = name.starts_with("quirin-provider-");
        if is_shared {
            shared += 1;
            // The per-library contract crates are data only, each usable on its
            // own (the libc crate in particular) without the shared bundle.
            // Card CONTRACT-INHERIT-ADOPTION (deliberate pin update): a data
            // crate may now depend on another data crate, because that Cargo
            // edge is the contract-use edge the published load path reads
            // (`#include` is admitted only along a declared dependency). The
            // edge set is pinned exactly below; nothing else is admitted, the
            // shared bundle is never a data crate's dependency, and libc stays
            // dependency-free.
            if name != "quirin-provider-shared" {
                for dependency in package["dependencies"].as_array().unwrap() {
                    let dependency_name = dependency["name"].as_str().unwrap();
                    assert!(
                        dependency_name.starts_with("quirin-provider-")
                            && dependency_name != "quirin-provider-shared",
                        "{name} is a data-only contract crate: {dependency_name}"
                    );
                    data_edges.push((name.to_owned(), dependency_name.to_owned()));
                }
            }
        }
        if is_artifact || is_shared {
            artifacts += usize::from(is_artifact);
            for dependency in package["dependencies"].as_array().unwrap() {
                let dependency_name = dependency["name"].as_str().unwrap();
                // Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING (deliberate pin
                // update): an artifact may also depend on a data contract crate,
                // because that Cargo edge is the contract-use edge the load
                // path reads; the executable links nothing from it. The exact
                // edge set is pinned below.
                let artifact_contract_edge = is_artifact
                    && dependency_name.starts_with("quirin-provider-")
                    && dependency_name != "quirin-provider-shared";
                if artifact_contract_edge {
                    artifact_contract_edges.push((name.to_owned(), dependency_name.to_owned()));
                }
                assert!(
                    matches!(dependency_name, "qcore-schemas" | "serde" | "serde_json")
                        || (is_artifact && dependency_name == "quirin-provider-shared")
                        || artifact_contract_edge
                        || (is_shared
                            && name != "quirin-provider-shared"
                            && dependency_name.starts_with("quirin-provider-")
                            && dependency_name != "quirin-provider-shared")
                        || (name == "quirin-provider-shared"
                            && dependency_name.starts_with("quirin-provider-")),
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
                    matches!(
                        name,
                        "qcore-schemas" | "serde" | "serde_json" | "quirin-provider-shared"
                    ),
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
    assert_eq!(shared, 6);
    assert_eq!(sdk, 1);
    // Card CONTRACT-INHERIT-ADOPTION: the one contract-use edge among the data
    // crates. Adding another is a deliberate edit here.
    data_edges.sort();
    assert_eq!(
        data_edges,
        [(
            "quirin-provider-fluent-bit".to_owned(),
            "quirin-provider-libc".to_owned()
        )]
    );
    // Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING: the two artifact crates
    // whose contracts inherit libc contracts, and nothing else.
    artifact_contract_edges.sort();
    assert_eq!(
        artifact_contract_edges,
        [
            (
                "qcore-provider-libevent".to_owned(),
                "quirin-provider-libc".to_owned()
            ),
            (
                "qcore-provider-librdkafka".to_owned(),
                "quirin-provider-libc".to_owned()
            ),
        ]
    );
}

/// The packaged artifacts are exactly the crates carrying a package template;
/// the conformance harness (which does link host crates for tests) is never one
/// of them, so no host crate can reach an installed provider package.
#[test]
fn only_template_carrying_crates_are_packaged() {
    let root = root();
    // Card W45-TEST-HYGIENE: a crate directory carrying a tracked template;
    // an untracked or ignored directory under `providers/` is not a crate.
    let mut templates: Vec<String> = tracked_files::tracked_files(&root)
        .iter()
        .filter_map(|path| {
            let relative = path.strip_prefix(&root).unwrap();
            let mut components = relative.components();
            let krate = components
                .next()?
                .as_os_str()
                .to_string_lossy()
                .into_owned();
            (components.as_path() == Path::new("provider-template.json")).then_some(krate)
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

/// Every shipped package answers identically under the target-qualified
/// **confined** execution profile, with the same kernel-enforced filesystem
/// and network confinement the adversarial fixtures exercise.
///
/// This is card P03's positive control against real binaries: the packages are
/// dynamically linked executables, so the admitted runtime roots are exactly
/// what they need, and nothing beyond them is reachable. The profile is
/// selected explicitly; the default profile is untouched.
#[test]
#[cfg(target_os = "linux")]
fn real_packages_answer_identically_under_the_confined_profile() {
    use qcore_verifier_host::{
        ConfinementPolicy, PathAccess, PathState, ProcessExecutionProfile, ProcessOutcome,
    };
    let root = root();
    let output = std::env::temp_dir().join(format!(
        "qcore-provider-confined-{}-{}",
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
        let build_confined = || {
            ProcessProvider::from_package(&package)
                .unwrap()
                .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
                .with_confinement_policy(ConfinementPolicy::linux_runtime_defaults())
        };
        let strict = build_confined();
        let manifest = strict.manifest().clone();
        let enforced = strict.enforced_limits().unwrap();
        // The confined profile composes with the limit profile rather than
        // replacing it: the same `setrlimit` plan is installed.
        assert_eq!(
            enforced.rlimits,
            ProcessProvider::from_package(&package)
                .unwrap()
                .with_execution_profile(ProcessExecutionProfile::LinuxRlimitV1)
                .enforced_limits()
                .unwrap()
                .rlimits
        );
        assert!(enforced.unenforced_kinds.is_empty());
        let confinement = enforced.confinement.as_ref().unwrap();
        assert_eq!(confinement.filesystem_mechanism, "landlock");
        assert!(confinement.landlock_abi >= confinement.landlock_min_abi);
        // The package root is admitted read-and-execute and nothing else in the
        // policy is writable except `/dev/null`.
        let package_root = confinement.paths.first().unwrap();
        assert_eq!(package_root.path, package.to_string_lossy());
        assert_eq!(package_root.access, PathAccess::ReadExecute);
        assert_eq!(package_root.state, PathState::Admitted);
        assert!(
            confinement
                .paths
                .iter()
                .filter(|path| path.access == PathAccess::ReadWrite)
                .all(|path| path.path == "/dev/null")
        );
        // No shipped manifest declares network access, and the confined profile
        // is what actually enforces that declaration.
        assert!(!confinement.network_access_declared);
        assert!(!manifest.network_access);
        for capability in &manifest.capabilities {
            let request = request(&manifest, *capability);
            assert_eq!(
                strict.verify(&request).unwrap(),
                legacy.verify(&request).unwrap()
            );
            let record = strict.last_execution().unwrap();
            assert_eq!(record.outcome, ProcessOutcome::Completed);
            assert_eq!(record.exit_code, Some(0));
            assert_eq!(record.profile, ProcessExecutionProfile::LinuxConfinedV1);
            assert!(record.summary().contains("linux-confined-v1"));
            assert!(record.summary().contains("landlock"));
            let (v2_request, task) = v2(&request);
            assert_eq!(
                strict.verify_v2(&v2_request).unwrap(),
                legacy.verify_v2(&v2_request).unwrap()
            );
            let mut registry = VerifierRegistry::default();
            registry.register(Arc::new(build_confined())).unwrap();
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

/// Card `P01-F3`: the spawn-time package-identity re-check, measured on the
/// five real shipped executables rather than on a shell fixture.
///
/// Two numbers matter and both are reported:
///
/// * the **cost** of the re-check — one SHA-256 over the executable plus one
///   `stat`, paid per dispatch;
/// * the **residual window** — from the completed re-check to `spawn`
///   returning, which is after the child has reported a successful `execve`.
///   That is an upper bound on the interval in which a writer could still
///   substitute the file, and it is deliberately *not* claimed to be zero.
///
/// The assertions are loose on purpose: this case exists to publish measured
/// numbers and to pin that every verified dispatch records one, not to pin a
/// machine's timing.
#[test]
#[cfg(unix)]
fn the_spawn_time_identity_recheck_is_measured_on_the_shipped_packages() {
    use qcore_verifier_host::ProcessOutcome;
    let root = root();
    let output = std::env::temp_dir().join(format!(
        "qcore-provider-identity-{}-{}",
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
    let mut measured = 0;
    for artifact in index["artifacts"].as_array().unwrap() {
        let id = artifact["plugin_id"].as_str().unwrap();
        let package = output.join(id);
        let provider =
            qcore_verifier_host::verify_executable_package(&package).expect("package verifies");
        let bytes = std::fs::metadata(package.join("provider")).unwrap().len();
        let manifest = provider.manifest().clone();
        let capability = *manifest.capabilities.iter().next().unwrap();
        let request = request(&manifest, capability);
        // The cost of the re-check itself, separately from the spawn: one
        // SHA-256 over the executable plus one `stat`.
        let verified = provider.verified_executable().expect("verified");
        let mut rechecks = Vec::new();
        for _ in 0..5 {
            let started = std::time::Instant::now();
            verified.revalidate(&manifest).expect("unchanged");
            rechecks.push(started.elapsed().as_micros() as u64);
        }
        rechecks.sort_unstable();
        let mut windows = Vec::new();
        let mut totals = Vec::new();
        for _ in 0..5 {
            let started = std::time::Instant::now();
            provider.verify(&request).expect("the package answers");
            let total = started.elapsed().as_micros() as u64;
            let record = provider.last_execution().unwrap();
            assert_eq!(record.outcome, ProcessOutcome::Completed);
            windows.push(
                record
                    .identity_window_us
                    .expect("every verified dispatch records its residual window"),
            );
            totals.push(total);
            measured += 1;
        }
        windows.sort_unstable();
        totals.sort_unstable();
        println!(
            "P01-F3 {id}: executable {bytes} bytes; residual window us min/median/max \
             {}/{}/{}; re-check us median {}; whole verify us median {}",
            windows[0],
            windows[windows.len() / 2],
            windows[windows.len() - 1],
            rechecks[rechecks.len() / 2],
            totals[totals.len() / 2]
        );
        // A window at all means the re-check ran; a window under a second on
        // any machine means it is the spawn, not a stall.
        assert!(windows[windows.len() - 1] < 1_000_000);

        // And the same provider refuses once the file is substituted, with
        // nothing spawned.
        let executable = package.join("provider");
        let original = std::fs::read(&executable).unwrap();
        std::fs::remove_file(&executable).unwrap();
        process_fixture::write_executable_mode(&executable, &original, 0o755);
        let failure = provider.verify(&request).unwrap_err();
        assert!(
            failure
                .to_string()
                .contains("was replaced since verification"),
            "{id}: {failure}"
        );
        assert_eq!(
            provider.last_execution().unwrap().outcome,
            ProcessOutcome::RefusedByIdentity
        );
        // The shipped bytes are unchanged, so the package still verifies from
        // scratch: what moved is file identity, not content.
        assert_eq!(
            sha256_hex(&std::fs::read(&executable).unwrap()),
            manifest.package_digest
        );
        qcore_verifier_host::verify_executable_package(&package)
            .expect("the restored package verifies again");
    }
    assert_eq!(measured, 25);
    std::fs::remove_dir_all(&output).unwrap();
}
