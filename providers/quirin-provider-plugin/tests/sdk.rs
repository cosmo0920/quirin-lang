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

//! Card W45-USER-CONTRACT-PLUGINS: the SDK on its example user crate
//! (`tests/fixtures/example-user-crate/quirin-provider-acme`) and its example
//! checker (`src/bin/example_checker.rs`).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_schemas::PLUGIN_EVIDENCE_VERSION_V2;
use qcore_schemas::plugin::{
    PLUGIN_PROTOCOL_VERSION, PluginId, ProposedVerdict, TargetProfile, UnitIdentity,
    VerificationRequestV2, VerificationResponseV2, VerifierCapability,
};
use quirin_provider_plugin::{load_path, package, template, user_crate::UserCrate};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "quirin-plugin-sdk-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/example-user-crate")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::write(&target, fs::read(entry.path()).unwrap()).unwrap();
        }
    }
}

#[test]
fn the_example_crate_reads_and_generates_its_user_load_path() {
    let krate = UserCrate::read(&example().join("quirin-provider-acme")).unwrap();
    assert_eq!(krate.name, "quirin-provider-acme");
    assert_eq!(krate.namespace, "acme");
    assert_eq!(krate.modules.keys().collect::<Vec<_>>(), ["acme/pool.qv"]);
    assert_eq!(krate.manifest.checkers.len(), 1);
    let manifest =
        load_path::user_load_path("../providers/contract-load-path.json", &example(), &[krate])
            .unwrap();
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["extends"], "../providers/contract-load-path.json");
    let record = &manifest["crates"][0];
    assert_eq!(record["contracts"], "quirin-provider-acme/contracts");
    assert_eq!(record["modules"], serde_json::json!(["acme/pool.qv"]));
    assert!(
        record.get("resource_view").is_none(),
        "the loader decides views"
    );
    assert_eq!(
        manifest["checkers"][0]["package"],
        "quirin-provider-acme/checkers/pool-check"
    );
    assert_eq!(manifest["overrides"], serde_json::json!([]));
}

#[test]
fn a_crate_outside_the_load_path_directory_or_malformed_is_refused() {
    let krate = UserCrate::read(&example().join("quirin-provider-acme")).unwrap();
    let elsewhere = scratch("elsewhere");
    assert!(load_path::user_load_path("x.json", &elsewhere, std::slice::from_ref(&krate)).is_err());
    assert!(
        load_path::user_load_path("x.json", &example(), &[krate.clone(), krate]).is_err(),
        "a crate listed twice"
    );
    for (file, edit, expected) in [
        (
            "Cargo.toml",
            ("quirin-provider-acme", "acme"),
            "quirin-provider-NAME",
        ),
        ("src/lib.rs", ("\"acme\";", "\"other\";"), "derived"),
        (
            "quirin-plugin.json",
            ("acme/pool.qv", "acme/absent.qcore"),
            "does not own",
        ),
        (
            "quirin-plugin.json",
            ("checkers/pool-check", "../pool-check"),
            "without traversal",
        ),
        (
            "quirin-plugin.json",
            (
                "\"acme_pool_get\", \"acme_pool_put\"",
                "\"acme_pool_put\", \"acme_pool_get\"",
            ),
            "sorted",
        ),
    ] {
        let root = scratch("malformed");
        let directory = root.join("quirin-provider-acme");
        copy_tree(&example().join("quirin-provider-acme"), &directory);
        let path = directory.join(file);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(edit.0), "{file}");
        fs::write(&path, text.replacen(edit.0, edit.1, 1)).unwrap();
        let error = UserCrate::read(&directory).unwrap_err();
        assert!(error.contains(expected), "{file}: {error}");
    }
    let root = scratch("stray");
    let directory = root.join("quirin-provider-acme");
    copy_tree(&example().join("quirin-provider-acme"), &directory);
    fs::write(directory.join("contracts/acme/notes.txt"), "x").unwrap();
    assert!(
        UserCrate::read(&directory)
            .unwrap_err()
            .contains("unexpected contract asset")
    );

    let root = scratch("alias");
    let directory = root.join("quirin-provider-acme");
    copy_tree(&example().join("quirin-provider-acme"), &directory);
    fs::copy(
        directory.join("contracts/acme/pool.qv"),
        directory.join("contracts/acme/pool.qcore"),
    )
    .unwrap();
    assert!(
        UserCrate::read(&directory)
            .unwrap_err()
            .contains("share one stem")
    );
}

#[test]
fn a_new_crate_from_the_template_is_a_valid_crate_held_for_review() {
    let root = scratch("template");
    let directory = root.join("quirin-provider-widget-pool");
    template::instantiate(&directory, "quirin-provider-widget-pool").unwrap();
    let krate = UserCrate::read(&directory).unwrap();
    assert_eq!(krate.namespace, "widget_pool");
    let text = &krate.modules["widget_pool/example.qv"];
    assert!(text.contains("extern fn widget_pool_release("));
    assert!(text.contains("requires resource_review_pending("));
    assert!(template::instantiate(&directory, "quirin-provider-widget-pool").is_err());
    assert!(template::instantiate(&root.join("x"), "widget").is_err());
}

#[test]
fn the_example_checker_packages_verifies_and_answers_a_claim() {
    let root = scratch("package");
    let output = root.join("pool-check");
    let installed = package::build(
        Path::new(env!("CARGO_BIN_EXE_quirin-plugin-example-checker")),
        &output,
    )
    .unwrap();
    assert_eq!(installed.plugin_id.as_str(), "acme.pool_check");
    assert_eq!(package::verify(&output).unwrap(), installed);
    // A package whose installed manifest was edited no longer verifies.
    let path = output.join(package::INSTALLED_MANIFEST);
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace("\"timeout_ms\": 5000", "\"timeout_ms\": 6000"),
    )
    .unwrap();
    assert!(package::verify(&output).unwrap_err().contains("differs"));
    fs::write(&path, text).unwrap();
    // The packaged checker answers one evidence-v2 frame with a Safe claim
    // and no evidence: what the audit's firewall reads as Pending.
    let request = VerificationRequestV2 {
        protocol_version: PLUGIN_PROTOCOL_VERSION,
        evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        request_id: "acme.pool_check:1".into(),
        plugin_id: PluginId::new("acme.pool_check").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "acme".into(),
            revision: "0.1.0".into(),
            profile: "user-contract".into(),
            dialect: "qcore-resource-v1".into(),
            platform: "any".into(),
            backend: "quirin-audit".into(),
        },
        unit: UnitIdentity {
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
            translation_unit: "quirin-provider-acme:acme/pool.qv".into(),
        },
        facts: Vec::new(),
        assumptions: Default::default(),
        task_digest: "c".repeat(64),
        obligations: vec![qcore_schemas::obligation::Obligation {
            obligation_id: "resource_contracts:0".into(),
            capability: VerifierCapability::ResourceContracts,
            property: qcore_schemas::obligation::ObligationProperty::ParameterModeRespected,
            subject: qcore_schemas::obligation::ObligationSubject::Function {
                symbol: "acme_pool_get".into(),
                translation_unit: "quirin-provider-acme:acme/pool.qv".into(),
            },
            scope: qcore_schemas::obligation::ObligationScope::WholeUnit,
            bounds: qcore_schemas::obligation::TaskBounds::single_path(),
            assumptions: Default::default(),
            evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        }],
        source_effect: None,
        counter_effect: None,
        reactor_program: None,
        teardown_source: None,
        call_use_source: None,
        cleanup_replay_source: None,
        composition_source: None,
    };
    let payload = serde_json::to_vec(&request).unwrap();
    let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
    frame.extend(payload);
    let mut child = Command::new(output.join("quirin-plugin-example-checker"))
        .current_dir(&output)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&frame).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let response: VerificationResponseV2 = serde_json::from_slice(&result.stdout[4..]).unwrap();
    assert_eq!(response.verdict, ProposedVerdict::Safe);
    assert!(response.evidence.is_none());
    assert_eq!(response.request_id, request.request_id);
}

/// Card PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS: `package::build`
/// called from a multi-threaded process while sibling threads fork and spawn
/// back to back. Each package is executed the moment `build` returns. If
/// `build` held a write descriptor on the executable, a sibling's forked,
/// not-yet-`exec`ed child could hold a duplicate of it, and this `execve`
/// would fail with `ETXTBSY` (`Text file busy`). The package must also be
/// today's package: the digest is the source's SHA-256, the installed
/// manifest is the embedded template with that digest, pretty-printed with a
/// final newline, and the executable is `0o755`.
#[test]
fn a_multi_threaded_caller_builds_and_executes_packages_while_siblings_spawn() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    let source = Path::new(env!("CARGO_BIN_EXE_quirin-plugin-example-checker"));
    let bytes = fs::read(source).unwrap();
    let digest = quirin_provider_plugin::sha256_hex(&bytes);
    let mut expected = package::embedded_template(&bytes).unwrap();
    expected.package_digest.clone_from(&digest);
    let mut expected_installed = serde_json::to_vec_pretty(&expected).unwrap();
    expected_installed.push(b'\n');

    let stop = Arc::new(AtomicBool::new(false));
    let spawned = Arc::new(AtomicUsize::new(0));
    let siblings: Vec<_> = (0..2)
        .map(|_| {
            let (stop, spawned) = (Arc::clone(&stop), Arc::clone(&spawned));
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let status = Command::new("/bin/true")
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                        .expect("a sibling spawns /bin/true");
                    assert!(status.success());
                    spawned.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect();
    // Both siblings are forking before the first build starts.
    while spawned.load(Ordering::Relaxed) < 2 && !siblings.iter().any(|s| s.is_finished()) {
        std::thread::yield_now();
    }
    assert!(
        !siblings.iter().any(|s| s.is_finished()),
        "a sibling stopped spawning"
    );

    let root = scratch("threads");
    let mut outcome = Ok(());
    for index in 0..4 {
        let output = root.join(format!("pool-check-{index}"));
        let installed = match package::build(source, &output) {
            Ok(installed) => installed,
            Err(error) => {
                outcome = Err(format!("build {index}: {error}"));
                break;
            }
        };
        let executable = output.join("quirin-plugin-example-checker");
        let run = Command::new(&executable)
            .arg("--verify-package")
            .current_dir(&output)
            .stdin(Stdio::null())
            .output();
        let run = match run {
            Ok(run) => run,
            Err(error) => {
                outcome = Err(format!(
                    "package {index} could not be executed right after build: {error} \
                     (raw os error {:?})",
                    error.raw_os_error()
                ));
                break;
            }
        };
        assert!(
            run.status.success(),
            "package {index}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        let reported: qcore_schemas::plugin::PluginManifest =
            serde_json::from_slice(&run.stdout).unwrap();
        assert_eq!(reported, installed, "package {index} reports its manifest");
        assert_eq!(installed, expected, "package {index}");
        assert_eq!(installed.package_digest, digest, "package {index}");
        assert_eq!(fs::read(&executable).unwrap(), bytes, "package {index}");
        assert_eq!(
            fs::read(output.join(package::INSTALLED_MANIFEST)).unwrap(),
            expected_installed,
            "package {index}: today's installed manifest bytes"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&executable).unwrap().permissions().mode() & 0o7777,
                0o755,
                "package {index}"
            );
        }
    }
    stop.store(true, Ordering::Relaxed);
    for sibling in siblings {
        sibling.join().unwrap();
    }
    outcome.unwrap();
    // Each package's bytes and manifest were compared above; the SDK's own
    // verification (in debug, a scan of the whole executable) runs once.
    assert_eq!(
        package::verify(&root.join("pool-check-0")).unwrap(),
        expected
    );
    fs::remove_dir_all(&root).unwrap();
}
