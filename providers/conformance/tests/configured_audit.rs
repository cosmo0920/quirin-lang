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

//! Real pinned checkout → configured audit entry point → schemas-only process.
//! No caller constructs imported summaries or expected witness paths.
#![cfg(unix)]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use qcore_audit::run_config;
use qcore_schemas::{PluginManifest, sha256_hex};
use serde_json::{Value, json};

// The provider executable is written by a separate process, so a sibling
// test's concurrent spawn cannot make its `execve` fail with `ETXTBSY`
// (`tests/support/process_fixture.rs`). Data files stay `fs::write`.
#[path = "../../../tests/support/process_fixture.rs"]
mod process_fixture;
use process_fixture::{copy_executable, write_executable_mode};

const SOURCE: &str = include_str!("fixtures/configured_boolean_source/sample.c");
const POLICY: &str = include_str!("fixtures/configured_boolean_source/sample.qv");
const PROFILE: &str = include_str!("fixtures/configured_boolean_source/sample-set/profile.json");
const RULES: &str = include_str!("fixtures/configured_boolean_source/source_projection.qv");
const HELPER: &str = include_str!("fixtures/configured_boolean_source/sample_reset.c");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    keep: bool,
}
impl Fixture {
    fn new(helper: bool) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-process-audit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("checkout/src")).unwrap();
        fs::create_dir(root.join("profile")).unwrap();
        let source = if helper {
            include_str!("fixtures/configured_boolean_source/sample_with_helper.c")
        } else {
            SOURCE
        };
        let source = format!(
            "{source}\nextern void opaque(void); void unsupported(void) {{opaque();}}\n#if 0\nint excluded(void) {{return 7;}}\n#endif\n"
        );
        fs::write(root.join("checkout/src/sample.c"), &source).unwrap();
        fs::write(root.join("checkout/src/sample_reset.c"), HELPER).unwrap();
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "src"],
            vec![
                "-c",
                "user.name=Audit fixture",
                "-c",
                "user.email=audit@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Pinned source-contract fixture",
            ],
        ] {
            let output = Command::new("git")
                .current_dir(root.join("checkout"))
                .args(args)
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
        }
        let output = Command::new("git")
            .current_dir(root.join("checkout"))
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        let revision = String::from_utf8(output.stdout).unwrap().trim().to_owned();
        let mut profile: Value = serde_json::from_str(PROFILE).unwrap();
        profile["source_set"] = json!("profile");
        profile["pins"]["sample"]["git_commit"] = json!(revision);
        profile["pins"]["sample"]["description"] = json!("Actual pinned local test checkout");
        profile["libraries"]["sample_widgets"]["source_roots"] =
            json!([{"root":"src","selection":{"exact":{"file":"src/sample.c"}}}]);
        profile["resource_contract_manifest"] = json!("../contracts.json");
        write_json(&root.join("profile/profile.json"), &profile);
        fs::write(
            root.join("alloc.qcore"),
            include_str!("../../../crates/qcore-audit/tests/fixtures/alloc_acquire.qv"),
        )
        .unwrap();
        write_json(
            &root.join("contracts.json"),
            &json!({"schema_version":1,"modules":["alloc.qcore"]}),
        );
        fs::write(
            root.join("policy.qcore"),
            if helper {
                include_str!("fixtures/configured_boolean_source/sample_with_helper.qv")
            } else {
                POLICY
            },
        )
        .unwrap();
        fs::write(root.join("rules.qcore"), RULES).unwrap();
        fs::write(
            root.join("frame.qcore"),
            include_str!("fixtures/configured_boolean_source/sample_reset.qv"),
        )
        .unwrap();
        let mut binding = json!({"library":"sample_widgets","provider":"qcore.sample_audit","contract":"policy.qcore"});
        if helper {
            binding["frame_helpers"] = json!([{"source":"sample_reset.c","function":"sample_reset","record":"sample_record","contract":"frame.qcore"}]);
        }
        write_json(
            &root.join("source-contracts.json"),
            &json!({"schema_version":1,"packages":["package"],"bindings":[binding]}),
        );
        fs::create_dir(root.join("package")).unwrap();
        copy_executable(
            Path::new(env!("CARGO_BIN_EXE_configured_audit_provider")),
            &root.join("package/provider"),
            0o755,
        );
        let mut manifest: PluginManifest = serde_json::from_str(include_str!(
            "fixtures/configured_boolean_source/audit-provider-template.json"
        ))
        .unwrap();
        manifest.package_digest = sha256_hex(&fs::read(root.join("package/provider")).unwrap());
        manifest.validate().unwrap();
        fs::write(
            root.join("package/verifier-plugin.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let database=["sample.c","sample_reset.c"].map(|name|json!({"directory":root.join("checkout"),"file":format!("src/{name}"),"arguments":["clang","-std=c11","-DSAMPLE_BUILD=1","-c",format!("src/{name}")]}));
        write_json(&root.join("compile_commands.json"), &json!(database));
        write_json(
            &root.join("audit.json"),
            &json!({
                "schema_version":1,"profile_directory":"profile","checkout":"checkout","compile_database":"compile_commands.json","libraries":["sample_widgets"],"report":"output/report.json",
                "source_contracts":{"configuration":"source-contracts.json","source_rules":"rules.qcore","libraries":[{"library":"sample_widgets","source_root":"src"}]},
            }),
        );
        Self { root, keep: false }
    }
    fn run(&self) -> Result<qcore_audit::projection::AuditCounts, String> {
        run_config(&self.root.join("audit.json"))
    }
    fn report(&self) -> Value {
        serde_json::from_slice(&fs::read(self.root.join("output/report.json")).unwrap()).unwrap()
    }
    fn mutate(&self, path: &str, change: impl FnOnce(&mut Value)) {
        let path = self.root.join(path);
        let mut v = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        change(&mut v);
        write_json(&path, &v);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

#[test]
fn pinned_second_codebase_runs_configured_process_checks_without_a_rust_import_adapter() {
    let mut fixture = Fixture::new(false);
    let counts = fixture.run().unwrap();
    assert_eq!(counts.units, 1);
    assert_eq!(counts.unknown_units, 1);
    let report = fixture.report();
    let effects = &report["source_contracts"];
    assert_eq!(effects["checked_function_contracts"], 2);
    assert_eq!(effects["unknown_functions"], 2);
    assert_eq!(report["units"][0]["status"], "Unknown");
    let functions = effects["units"][0]["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 4);
    for name in ["unsupported", "excluded"] {
        assert_eq!(
            functions.iter().find(|v| v["function"] == name).unwrap()["status"],
            "unknown"
        );
    }
    for f in functions
        .iter()
        .filter(|f| f["status"] == "checked_function_contract")
    {
        assert!(f["witness"].is_object());
        assert!(f["task"].is_object());
        assert_eq!(f["checked_states"], 4);
        assert_eq!(
            f["import_identity"]["source_revision"],
            effects["selection"][0]["revision"]
        );
    }
    fixture.mutate("audit.json", |v| {
        v.as_object_mut().unwrap().remove("source_contracts");
    });
    let plain = fixture.run().unwrap();
    assert_eq!(counts.unknown_units, plain.unknown_units);
    let mut plain_report = fixture.report();
    let mut augmented = report.clone();
    augmented
        .as_object_mut()
        .unwrap()
        .remove("source_contracts");
    // The augmented configuration has different compile-task dependencies.
    // Graph contexts must bind that distinction; erasing evidence must still
    // reproduce every original custody verdict, diagnostic and report field.
    assert_ne!(
        augmented["units"][0]["functions"][0]["hypothesis_context"]["build_identity"],
        plain_report["units"][0]["functions"][0]["hypothesis_context"]["build_identity"]
    );
    qcore_audit::contract_dependencies::erase_hypotheses(&mut augmented);
    qcore_audit::contract_dependencies::erase_hypotheses(&mut plain_report);
    assert_eq!(augmented, plain_report);
    if let Ok(path) = std::env::var("QCORE_KEEP_PROCESS_AUDIT_FIXTURE") {
        fixture.mutate("audit.json",|v|v["source_contracts"]=json!({"configuration":"source-contracts.json","source_rules":"rules.qcore","libraries":[{"library":"sample_widgets","source_root":"src"}]}));
        fs::write(path, fixture.root.to_string_lossy().as_bytes()).unwrap();
        fixture.keep = true;
    }
}

#[test]
fn configured_helper_is_imported_from_its_real_pinned_compile_entry() {
    let fixture = Fixture::new(true);
    fixture.run().unwrap();
    let report = fixture.report();
    let effects = &report["source_contracts"];
    assert_eq!(effects["checked_function_contracts"], 1);
    assert_eq!(effects["unknown_functions"], 2);
    assert_eq!(effects["frame_helpers"].as_array().unwrap().len(), 1);
    assert_eq!(effects["frame_helpers"][0]["function"], "sample_reset");
    assert_eq!(
        effects["frame_helpers"][0]["import_identity"]["raw_source_digest"],
        sha256_hex(HELPER.as_bytes())
    );
}

#[test]
fn failed_contract_evidence_is_unknown_without_resource_verdict_promotion() {
    let fixture = Fixture::new(false);
    let policy = POLICY
        .replace("result == old_lock", "result == 7")
        .replace("result == 1 - old_lock", "result == 7");
    fs::write(fixture.root.join("policy.qcore"), policy).unwrap();
    fixture.run().unwrap();
    let report = fixture.report();
    assert_eq!(report["source_contracts"]["checked_function_contracts"], 0);
    assert_eq!(report["source_contracts"]["unknown_functions"], 4);
    assert_eq!(report["units"][0]["status"], "Unknown");
    for f in report["source_contracts"]["units"][0]["functions"]
        .as_array()
        .unwrap()
    {
        assert_eq!(f["status"], "unknown");
        assert!(f.get("witness").is_none());
    }
}

#[test]
fn missing_bindings_models_packages_and_escaping_roots_do_not_publish_a_report() {
    for problem in [
        "binding",
        "library",
        "model",
        "package",
        "root",
        "flags",
        "duplicate",
    ] {
        let fixture = Fixture::new(false);
        match problem {
            "binding" => fixture.mutate("source-contracts.json", |v| {
                v["bindings"][0]["library"] = json!("other")
            }),
            "library" => fixture.mutate("audit.json", |v| {
                v["source_contracts"]["libraries"][0]["library"] = json!("other")
            }),
            "model" => fixture.mutate("profile/profile.json", |v| {
                v["libraries"]["sample_widgets"]["boolean_field_models"] = json!([])
            }),
            "package" => {
                fs::remove_file(fixture.root.join("package/provider")).unwrap();
            }
            "root" => fixture.mutate("audit.json", |v| {
                v["source_contracts"]["libraries"][0]["source_root"] = json!("..")
            }),
            "flags" => fixture.mutate("compile_commands.json", |v| {
                v[0]["arguments"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("-fpack-struct"))
            }),
            "duplicate" => fixture.mutate("audit.json", |v| {
                let first = v["source_contracts"]["libraries"][0].clone();
                v["source_contracts"]["libraries"]
                    .as_array_mut()
                    .unwrap()
                    .push(first);
            }),
            _ => unreachable!(),
        }
        assert!(fixture.run().is_err(), "{problem}");
        assert!(!fixture.root.join("output/report.json").exists());
    }
}

#[test]
fn changed_main_or_helper_source_cannot_borrow_a_pinned_revision() {
    for name in ["sample.c", "sample_reset.c"] {
        let fixture = Fixture::new(true);
        fixture.mutate("profile/profile.json", |v| {
            v["pins"]["sample"]["clean_paths"] = json!(["unrelated"])
        });
        let path = fixture.root.join("checkout/src").join(name);
        let source = fs::read_to_string(&path).unwrap();
        fs::write(path, format!("{source}\n/* unpinned change */\n")).unwrap();
        let error = fixture.run().err().expect("unpinned source must reject");
        assert!(error.contains("does not match its pinned bytes"), "{error}");
        assert!(!fixture.root.join("output/report.json").exists());
    }
}

#[test]
fn a_failed_selected_process_stays_unknown_without_an_in_process_fallback() {
    let fixture = Fixture::new(false);
    // The host verifies this package snapshot — digest, envelope and every
    // declaration bind — and the process then fails at dispatch. Since P01-F2
    // the failure cannot be induced by editing the manifest, so it is induced
    // where only execution can see it: an executable that carries the very
    // template the manifest declares and refuses to answer.
    let executable = fixture.root.join("package/provider");
    let manifest_path = fixture.root.join("package/verifier-plugin.json");
    let mut manifest: PluginManifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    let mut template = manifest.clone();
    template.package_digest = "0".repeat(64);
    let mut bytes = b"#!/bin/sh\nexit 3\n".to_vec();
    bytes.extend(
        qcore_verifier_host::envelope(&serde_json::to_string_pretty(&template).unwrap())
            .into_bytes(),
    );
    write_executable_mode(&executable, &bytes, 0o755);
    manifest.package_digest = sha256_hex(&bytes);
    write_json(&manifest_path, &serde_json::to_value(&manifest).unwrap());
    fixture.run().unwrap();
    let report = fixture.report();
    assert_eq!(report["source_contracts"]["checked_function_contracts"], 0);
    assert_eq!(report["source_contracts"]["unknown_functions"], 4);
    for name in ["sample_ready", "sample_toggle"] {
        let function = report["source_contracts"]["units"][0]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["function"] == name)
            .unwrap();
        assert_eq!(function["status"], "unknown");
        assert!(
            function["reason"]
                .as_str()
                .unwrap()
                .contains("was not admitted")
        );
        assert!(function.get("witness").is_none());
    }
    assert_eq!(report["units"][0]["status"], "Unknown");
}

#[test]
fn missing_or_conflicting_helper_compile_entry_rejects_before_publication() {
    for duplicate in [false, true] {
        let fixture = Fixture::new(true);
        fixture.mutate("compile_commands.json", |v| {
            if duplicate {
                let mut entry = v[1].clone();
                entry["arguments"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("-DOTHER=1"));
                v.as_array_mut().unwrap().push(entry);
            } else {
                v.as_array_mut().unwrap().pop();
            }
        });
        assert!(fixture.run().is_err());
        assert!(!fixture.root.join("output/report.json").exists());
    }
}
