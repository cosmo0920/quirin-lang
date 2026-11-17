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

//! Card W48-OPENSSL-CONTRACT-PLUGIN: the plugin through the shipped host
//! route (`qcore_audit::subcommand_plugins::main`, the code behind
//! `quirin-audit openssl …`), so the firewall, the join and the report are
//! the ones a user gets. Fail-closed rows: a pin mismatch or an absent tree
//! (every binding `Unknown`), a tarball that is not the pinned one, a usage
//! error and a timeout (the run `Unknown`, its claims joined nowhere), and a
//! host `Unsafe` row that the plugin does not lower. The positive run over
//! the real pinned release needs its tarball and is ignored without it.
//!
//! Unix only, like every host-route test: on another host the host spawns no
//! external subcommand (its quota kill is a process-group kill), so every run
//! fails closed as `plugin-spawn` before the plugin is reached.

#![cfg(unix)]

mod support;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::Value;

const PLUGIN: &str = env!("CARGO_BIN_EXE_quirin-audit-openssl");

static HOST: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn host(output: &Path, extra: &[&str], arguments: &[&str]) -> (ExitCode, Value) {
    let _serial = HOST
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let plugins = Path::new(PLUGIN).parent().unwrap();
    let mut argv = vec![
        "--plugin-path".to_owned(),
        plugins.display().to_string(),
        "--output".to_owned(),
        output.display().to_string(),
    ];
    argv.extend(extra.iter().map(|word| (*word).to_owned()));
    argv.push("openssl".to_owned());
    argv.extend(arguments.iter().map(|word| (*word).to_owned()));
    let code = qcore_audit::subcommand_plugins::main(argv);
    let report = output.join("quirin-audit-openssl.report.json");
    let report: Value = serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
    (code, report)
}

fn rows(report: &Value) -> Vec<&Value> {
    report["rows"].as_array().unwrap().iter().collect()
}

fn count(report: &Value, verdict: &str) -> usize {
    rows(report)
        .iter()
        .filter(|row| row["verdict"] == verdict)
        .count()
}

#[test]
fn without_a_prepared_tree_every_binding_is_unknown() {
    let directory = support::scratch("absent");
    let clang = support::clang();
    let (code, report) = host(&directory, &[], &["check", "--clang", &clang]);
    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(report["run"]["outcome"], "completed");
    assert_eq!(rows(&report).len(), 68);
    assert_eq!(count(&report, "Unknown"), 68);
    assert_eq!(report["verdict"], "Unknown");
    let diagnostics = report["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "openssl-pin-mismatch")
    );
    // Every row names the binding and keeps the plugin's reason.
    let first = rows(&report)
        .into_iter()
        .find(|row| row["artifact"] == "openssl@3.5.5::ERR_clear_error")
        .unwrap();
    assert!(
        first["reasons"][0]["detail"]
            .as_str()
            .unwrap()
            .contains("release-pin")
    );
}

#[test]
fn a_tree_that_is_not_the_pinned_release_makes_every_binding_unknown() {
    let directory = support::scratch("tampered");
    let tree = directory.join("openssl-3.5.5");
    std::fs::create_dir_all(&tree).unwrap();
    std::fs::write(
        tree.join("VERSION.dat"),
        "MAJOR=3\nMINOR=5\nPATCH=5\nPRE_RELEASE_TAG=\n",
    )
    .unwrap();
    let clang = support::clang();
    let (code, report) = host(&directory, &[], &["check", "--clang", &clang]);
    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(count(&report, "Unknown"), 68);
    assert_eq!(count(&report, "Pending"), 0);
    assert_eq!(count(&report, "Safe"), 0);
    let record: Value =
        serde_json::from_slice(&std::fs::read(directory.join("openssl-check.json")).unwrap())
            .unwrap();
    assert_eq!(record["pin"]["held"], false);
    assert!(
        record["pin"]["detail"]
            .as_str()
            .unwrap()
            .contains("not the pinned")
    );
}

#[test]
fn a_tarball_that_is_not_the_pinned_release_fails_the_run() {
    let directory = support::scratch("tarball");
    let tarball = directory.join("openssl-3.5.5.tar.gz");
    std::fs::write(&tarball, b"not the release").unwrap();
    let output = directory.join("out");
    let (code, report) = host(
        &output,
        &[],
        &["prepare", "--tarball", tarball.to_str().unwrap()],
    );
    assert_eq!(code, ExitCode::FAILURE);
    assert_eq!(report["run"]["outcome"], "failed");
    assert_eq!(report["verdict"], "Unknown");
    assert!(
        report["run"]["reason"]
            .as_str()
            .unwrap()
            .contains("release pin mismatch")
    );
    assert!(!output.join("openssl-3.5.5").exists());
}

#[test]
fn a_usage_error_fails_the_run() {
    let directory = support::scratch("usage");
    let (code, report) = host(&directory, &[], &["check", "--binding", "NOT_A_BINDING"]);
    assert_eq!(code, ExitCode::FAILURE);
    assert_eq!(report["verdict"], "Unknown");
    assert!(rows(&report).iter().all(|row| row["verdict"] != "Safe"));
    let (code, report) = host(&directory, &[], &["prepare"]);
    assert_eq!(code, ExitCode::FAILURE);
    assert_eq!(report["verdict"], "Unknown");
}

#[test]
fn a_timeout_makes_the_run_unknown_and_joins_no_claim() {
    let directory = support::scratch("timeout");
    // A "Clang" that never answers: the plugin's first step blocks, so the
    // host's 300 ms quota is breached however fast the machine is. The
    // script is written by a separate process (no descriptor of ours is open
    // on it when it is executed: the ETXTBSY shape of constraint 16).
    let stalled = directory.join("stalled-clang");
    let status = std::process::Command::new("sh")
        .args([
            "-c",
            "printf '#!/bin/sh\\nexec sleep 60\\n' > \"$1\" && chmod 755 \"$1\"",
            "sh",
            stalled.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let (code, report) = host(
        &directory,
        &["--timeout-ms", "300"],
        &["check", "--clang", stalled.to_str().unwrap()],
    );
    assert_eq!(code, ExitCode::FAILURE);
    assert_eq!(report["run"]["outcome"], "failed");
    assert_eq!(report["run"]["code"], "plugin-timeout");
    assert_eq!(report["verdict"], "Unknown");
    assert!(rows(&report).iter().all(|row| row["verdict"] == "Unknown"));
}

#[test]
fn a_host_unsafe_row_is_not_lowered_by_the_plugin() {
    let directory = support::scratch("host-row");
    let audit = directory.join("audit.json");
    std::fs::write(
        &audit,
        serde_json::to_vec(&serde_json::json!({"units": [
            {"file": "openssl@3.5.5::ERR_clear_error", "status": "Unsafe",
             "reason": "a host row", "reason_code": "residual-custody"}
        ]}))
        .unwrap(),
    )
    .unwrap();
    let output = directory.join("out");
    let clang = support::clang();
    let (code, report) = host(
        &output,
        &["--audit-report", audit.to_str().unwrap()],
        &["check", "--clang", &clang],
    );
    assert_eq!(code, ExitCode::SUCCESS);
    let row = rows(&report)
        .into_iter()
        .find(|row| row["artifact"] == "openssl@3.5.5::ERR_clear_error")
        .unwrap();
    assert_eq!(row["verdict"], "Unsafe");
    assert_eq!(report["verdict"], "Unsafe");
}

fn tarball() -> Option<PathBuf> {
    std::env::var_os("QUIRIN_OPENSSL_TARBALL").map(PathBuf::from)
}

/// The positive run over the real pinned release (card report, section
/// "Check"): prepare, then check every binding. Every `Safe` proposal is
/// published `Pending`, no row is `Safe`, `ERR_clear_error` is `Pending`, and
/// the refcounted releasers are `Unknown`.
#[test]
#[ignore = "needs the pinned OpenSSL 3.5.5 release tarball in QUIRIN_OPENSSL_TARBALL"]
fn the_pinned_release_prepares_and_checks() {
    let tarball = tarball().expect("QUIRIN_OPENSSL_TARBALL");
    let directory = support::scratch("release");
    let clang = support::clang();
    let (code, report) = host(
        &directory,
        &[],
        &[
            "prepare",
            "--tarball",
            tarball.to_str().unwrap(),
            "--clang",
            &clang,
            "--jobs",
            "8",
        ],
    );
    assert_eq!(code, ExitCode::SUCCESS, "{report:#}");
    let (code, report) = host(&directory, &[], &["check", "--clang", &clang]);
    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(count(&report, "Safe"), 0);
    assert_eq!(count(&report, "Pending"), 16);
    assert_eq!(count(&report, "Unknown"), 52);
    let verdict = |function: &str| {
        rows(&report)
            .into_iter()
            .find(|row| row["artifact"] == format!("openssl@3.5.5::{function}"))
            .unwrap()["verdict"]
            .clone()
    };
    assert_eq!(verdict("ERR_clear_error"), "Pending");
    assert_eq!(verdict("X509_free"), "Unknown");
    assert_eq!(verdict("SSL_get_certificate"), "Unknown");
}
