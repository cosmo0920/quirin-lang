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

//! Regression for the current route and explicit pinned replay boundary.

use serde_json::{Value, json};
use std::fs;
use std::process::Command;

#[test]
fn current_cycle19_route_accepts_a_new_revision_and_replaces_old_checked_sidecars() {
    if let Err(reason) = qcore_c_gateway::clang_command() {
        assert!(
            std::env::var_os("QCORE_CLANG").is_none(),
            "explicit QCORE_CLANG is invalid: {reason}"
        );
        eprintln!("skipping Clang-dependent Cycle 19 command test: {reason}");
        return;
    }
    let clang = qcore_c_gateway::clang_command().unwrap();
    let compiler = clang.get_program().to_str().unwrap();
    let root = std::env::temp_dir().join(format!("qcore-cycle19-command-{}", std::process::id()));
    fs::create_dir_all(root.join("build")).unwrap();
    let files = [
        "lib/cmetrics/src/unit.c",
        "lib/ctraces/src/unit.c",
        "lib/librdkafka-2.15.0/src/unit.c",
        "lib/chunkio/src/unit.c",
        "src/flb_input_chunk.c",
    ];
    for file in files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "int entry(void) { return 1; }").unwrap();
    }
    for args in [
        vec!["init", "--quiet"],
        vec!["add", "lib", "src"],
        vec![
            "-c",
            "user.name=Audit Fixture",
            "-c",
            "user.email=audit@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "Current revision fixture",
        ],
    ] {
        let output = Command::new("git")
            .current_dir(&root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fs::write(
        root.join("build/compile_commands.json"),
        json!(
            files
                .iter()
                .map(|file| json!({
                    "directory": root, "file": file, "arguments": [compiler, "-std=c11", "-c", file]
                }))
                .collect::<Vec<_>>()
        )
        .to_string(),
    )
    .unwrap();
    let report = root.join("reports/resources.json");
    let monkey = root.join("reports/monkey.json");
    let bindings = root.join("observations");
    fs::create_dir_all(report.parent().unwrap()).unwrap();
    fs::create_dir_all(&bindings).unwrap();
    for path in [
        report.with_extension("chunk-contracts.json"),
        report.with_extension("source-effects.json"),
        monkey.clone(),
        bindings.join("monkey_binding_result.json"),
    ] {
        fs::write(path, r#"{"status":"Checked","source_revision":"old"}"#).unwrap();
    }
    let command = |report: &std::path::Path,
                   monkey: Option<&std::path::Path>,
                   bindings: Option<&std::path::Path>,
                   database: Option<&std::path::Path>,
                   jobs: &str| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
        command
            .args(["current", "--checkout"])
            .arg(&root)
            .arg("--profile-directory")
            .arg(quirin_source_analysis::source_profiles::profiles_root().join("fluent-bit"))
            .arg("--report")
            .arg(report)
            .args(["--jobs", jobs]);
        if let Some(path) = monkey {
            command.arg("--monkey-source-report").arg(path);
        }
        if let Some(path) = bindings {
            command.arg("--binding-directory").arg(path);
        }
        if let Some(path) = database {
            command.arg("--compile-database").arg(path);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    command(&report, Some(&monkey), Some(&bindings), None, "1");
    // Card W44-AUDIT-PARALLEL-UNITS: the same audit on four threads writes the
    // same report and assumptions bytes as the sequential run.
    let parallel = root.join("reports-parallel/resources.json");
    fs::create_dir_all(parallel.parent().unwrap()).unwrap();
    command(&parallel, None, None, None, "4");
    for extension in ["json", "contracts.json"] {
        assert_eq!(
            fs::read(parallel.with_extension(extension)).unwrap(),
            fs::read(report.with_extension(extension)).unwrap(),
            "{extension}"
        );
    }
    let result: Value = serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
    assert_ne!(
        result["fluent_bit_revision"],
        quirin_source_analysis::chunkio::PINNED_FLUENT_BIT_REVISION
    );
    assert_eq!(result["units"].as_array().unwrap().len(), 5);
    assert!(
        result["units"]
            .as_array()
            .unwrap()
            .iter()
            .all(|unit| unit["status"] == "Safe")
    );
    // A separately prepared Clang database must work without changing the
    // developer's default build directory or copying its generated database.
    let alternate = root.join("clang-compile-commands.json");
    fs::rename(root.join("build/compile_commands.json"), &alternate).unwrap();
    fs::create_dir_all(root.join("reports-alternate")).unwrap();
    command(
        &root.join("reports-alternate/resources.json"),
        None,
        None,
        Some(&alternate),
        "2",
    );
    fs::rename(&alternate, root.join("build/compile_commands.json")).unwrap();
    for path in [
        report.with_extension("chunk-contracts.json"),
        report.with_extension("source-effects.json"),
        monkey,
        bindings.join("monkey_binding_result.json"),
    ] {
        let evidence: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(evidence["status"], "Unknown");
        assert_eq!(evidence["source_revision"], result["fluent_bit_revision"]);
    }
    let output = Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
        .args(["pinned", "--jobs", "1"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("Fluent Bit pin check failed"), "{error}");
    fs::remove_dir_all(root).unwrap();
}
