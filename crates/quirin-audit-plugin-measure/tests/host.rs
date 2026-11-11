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

//! Card MEASURE-TOOL: `quirin-audit measure` through the real host, behind
//! the subcommand-plugin firewall and its receipts (contract specification
//! C11.2): the record is a verified artifact, the tool claims nothing, and a
//! failed measurement is an `Unknown` run with its invocation receipt.

#![cfg(unix)]

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use support::*;

/// The host binary beside the plugin in the same target directory, built
/// on demand (a scoped gate may build this crate alone).
fn host() -> PathBuf {
    let directory = Path::new(PLUGIN).parent().unwrap();
    let host = directory.join("quirin-audit");
    if !host.is_file() {
        let mut command = Command::new(env!("CARGO"));
        command.args([
            "build",
            "--offline",
            "-p",
            "qcore-audit",
            "--bin",
            "quirin-audit",
        ]);
        if directory.file_name().is_some_and(|name| name == "release") {
            command.arg("--release");
        }
        let status = command
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .unwrap();
        assert!(status.success(), "building the host failed");
    }
    host
}

fn host_run(
    directory: &Path,
    output: &str,
    arguments: &[&str],
) -> (std::process::Output, Value, Value) {
    let plugins = Path::new(PLUGIN).parent().unwrap().display().to_string();
    let mut all = vec![
        "--plugin-path",
        plugins.as_str(),
        "--output",
        output,
        "measure",
    ];
    all.extend_from_slice(arguments);
    let result = Command::new(host())
        .args(&all)
        .current_dir(directory)
        .env_remove("QUIRIN_AUDIT_PLUGIN_PATH")
        .output()
        .unwrap();
    let read = |name: &str| -> Value {
        serde_json::from_slice(&std::fs::read(directory.join(output).join(name)).unwrap()).unwrap()
    };
    (
        result,
        read("quirin-audit-measure.report.json"),
        read("quirin-audit-measure.report.invocation.json"),
    )
}

#[test]
fn the_record_is_a_verified_artifact_and_the_tool_claims_no_verdict() {
    let directory = scratch("host-ok");
    write_route(
        &directory.join("a"),
        "slice",
        &before_units(),
        Some("analyzer@1"),
    );
    write_route(
        &directory.join("b"),
        "slice",
        &after_units(),
        Some("analyzer@2"),
    );
    let manifest = manifest(&directory).display().to_string();
    let (result, report, invocation) = host_run(
        &directory,
        "out",
        &[
            "--inputs",
            &manifest,
            "--side",
            "x=run:a",
            "--side",
            "y=run:b",
            "--no-slots",
        ],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    assert_eq!(report["run"]["outcome"], "completed");
    assert_eq!(report["claims"], Value::Array(Vec::new()));
    assert_eq!(
        report["verdict"],
        Value::Null,
        "no claim, no host row: no verdict"
    );
    let artifact = report["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|artifact| artifact["kind"] == "native-comparison")
        .expect("the record is an artifact");
    let bytes = std::fs::read(directory.join("out/native-comparison.json")).unwrap();
    assert_eq!(artifact["sha256"], qcore_schemas::sha256_hex(&bytes));
    assert_eq!(invocation["outcome"], "completed");
    assert_eq!(invocation["environment"]["cleared"], true);
    assert!(
        invocation["plugin"]["sha256"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn a_failed_measurement_is_an_unknown_run_with_its_receipt() {
    let directory = scratch("host-failed");
    write_route(
        &directory.join("a"),
        "slice",
        &before_units(),
        Some("analyzer@1"),
    );
    write_route(
        &directory.join("b"),
        "slice",
        &after_units(),
        Some("analyzer@2"),
    );
    let manifest = manifest(&directory).display().to_string();
    let prediction = directory.join("PREDICTION.md");
    std::fs::write(
        &prediction,
        "```measure-prediction\nroute slice\nnone\n```\n",
    )
    .unwrap();
    let prediction = prediction.display().to_string();
    let (result, report, invocation) = host_run(
        &directory,
        "out",
        &[
            "--inputs",
            &manifest,
            "--side",
            "x=run:a",
            "--side",
            "y=run:b",
            "--no-slots",
            "--declared",
            &prediction,
        ],
    );
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(report["run"]["outcome"], "failed");
    assert_eq!(report["run"]["verdict"], "Unknown");
    assert!(
        report["run"]["reason"]
            .as_str()
            .unwrap()
            .contains("declared-versus-measured mismatch")
    );
    assert_eq!(
        report["artifacts"],
        Value::Array(Vec::new()),
        "a failed run's files are not accepted"
    );
    assert_eq!(invocation["outcome"], "failed");
    // The record is still on disk for the person, naming the unpredicted rows.
    let record: Value = serde_json::from_slice(
        &std::fs::read(directory.join("out/native-comparison.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(record["declared"]["result"], "mismatch");
    assert_eq!(
        record["declared"]["routes"]["slice"]["unpredicted"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
