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

//! Built-in pinned command parses before any expensive source replay.
use std::process::Command;

const HOST: &str = env!("CARGO_BIN_EXE_quirin-audit");

#[test]
fn pinned_help_and_invalid_options_do_not_start_a_replay() {
    let help = Command::new(HOST)
        .args(["pinned", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("usage: quirin-audit pinned"));
    for args in [
        vec!["pinned", "--not-a-flag"],
        vec!["pinned", "--jobs", "1", "--jobs", "2"],
        vec!["pinned", "--jobs", "0"],
        vec!["pinned", "/tmp", "--not-a-flag"],
        // Card CORPUS-GROUP-CHECKPOINTS: the checkpoint options.
        vec!["pinned", "--resume"],
        vec!["pinned", "--checkpoint-dir"],
        vec!["pinned", "--checkpoint-dir", ""],
        vec!["pinned", "--checkpoint-dir", "a", "--checkpoint-dir", "b"],
        vec!["pinned", "--checkpoint-dir", "a", "--resume", "--resume"],
        // Card MEASURE-LOOKUP-RECORD-REUSE: the analyzer acceptance options.
        vec![
            "pinned",
            "--checkpoint-dir",
            "a",
            "--accept-base-analyzer",
            "HEAD",
        ],
        vec!["pinned", "--checkpoint-dir", "a", "--repository", "."],
        vec![
            "pinned",
            "--accept-base-analyzer",
            "HEAD",
            "--repository",
            ".",
        ],
        vec![
            "pinned",
            "--checkpoint-dir",
            "a",
            "--accept-base-analyzer",
            "HEAD",
            "--accept-base-analyzer",
            "HEAD",
        ],
        vec!["pinned", "--checkpoint-dir", "a", "--repository", ""],
    ] {
        let output = Command::new(HOST).args(&args).output().unwrap();
        assert!(!output.status.success(), "{args:?}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!error.contains("could not resolve"), "{args:?}: {error}");
    }
}

/// Card MEASURE-LOOKUP-RECORD-REUSE: a base analyzer claimed for a diff
/// that is not confined to contract data is refused before any replay.
/// Skipped where this checkout's history is absent.
#[test]
fn a_base_analyzer_for_a_non_contract_diff_is_refused_before_any_replay() {
    let repository = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let present = Command::new("git")
        .args([
            "-C",
            repository,
            "rev-parse",
            "--verify",
            "74105dd0e~1^{commit}",
        ])
        .output()
        .is_ok_and(|output| output.status.success());
    if !present {
        return;
    }
    let output = Command::new(HOST)
        .args([
            "pinned",
            "--checkpoint-dir",
            "never-created",
            "--accept-base-analyzer",
            "74105dd0e~1",
            "--repository",
            repository,
            "/nonexistent-root",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("analyzer acceptance refused: the diff is not confined to contract data"),
        "{error}"
    );
    assert!(!error.contains("could not resolve"), "{error}");
    assert!(!std::path::Path::new("never-created").exists());
}

/// Card CORPUS-GROUP-CHECKPOINTS: an environment a checkpoint cannot stand
/// for (here a per-unit side output) is refused before any input is read.
#[test]
fn pinned_checkpoints_refuse_unsupported_environments_before_any_replay() {
    for (variable, value) in [
        ("QCORE_UNIT_TIMING", "timing.jsonl"),
        ("QCORE_TYPED_PATH_PARITY", "parity.jsonl"),
        ("QCORE_BOUNDS_PROPERTY", "bounds.jsonl"),
        ("QCORE_SOURCE_CONTRACTS", "source-contracts.json"),
    ] {
        let output = Command::new(HOST)
            .args([
                "pinned",
                "--checkpoint-dir",
                "never-created",
                "/nonexistent-root",
            ])
            .env(variable, value)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{variable}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("pinned checkpoints refuse {variable}")),
            "{variable}: {error}"
        );
        assert!(!std::path::Path::new("never-created").exists());
    }
}

/// Card MEASURE-CONTRACT-ONLY-SIDE-WIRING: `pinned --classify-only` answers
/// the classification a run claiming the base analyzer would get, as one JSON
/// line or a refusal, and takes no run options.
#[test]
fn classify_only_answers_the_contract_only_classification_without_a_run() {
    let directory = std::env::temp_dir().join(format!(
        "qcore-classify-only-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(directory.join("contract/m")).unwrap();
    std::fs::create_dir_all(directory.join("crates/a/src")).unwrap();
    std::fs::write(directory.join("contract/m/x.qv"), "module contract;\n").unwrap();
    std::fs::write(directory.join("crates/a/src/lib.rs"), "pub fn a() {}\n").unwrap();
    let git = |arguments: &[&str]| {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(&directory)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {arguments:?}");
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    let commit = |message: &str| {
        git(&["add", "-A"]);
        git(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "-m",
            message,
        ]);
        git(&["rev-parse", "HEAD"])
    };
    git(&["init", "-q"]);
    let base = commit("base");
    std::fs::write(
        directory.join("contract/m/x.qv"),
        "module contract;\n// c\n",
    )
    .unwrap();
    let tip = commit("contract only");
    let classify = |base: &str| {
        Command::new(HOST)
            .args(["pinned", "--classify-only", "--accept-base-analyzer", base])
            .arg("--repository")
            .arg(&directory)
            .output()
            .unwrap()
    };
    let accepted = classify(&base);
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
    let line: serde_json::Value = serde_json::from_slice(&accepted.stdout).unwrap();
    assert_eq!(line["schema"], "quirin-contract-only-classification/1");
    assert_eq!(line["accepted"], true);
    assert_eq!(line["acceptance"]["base"], base.as_str());
    assert_eq!(line["acceptance"]["tip"], tip.as_str());
    assert_eq!(
        line["acceptance"]["paths"],
        serde_json::json!(["contract/m/x.qv"])
    );
    assert_eq!(line["decision"]["accepted_from"], "base");
    std::fs::write(directory.join("crates/a/src/lib.rs"), "pub fn b() {}\n").unwrap();
    commit("analyzer source");
    let refused = classify(&base);
    assert!(!refused.status.success());
    assert!(refused.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&refused.stderr)
            .contains("analyzer acceptance refused: the diff is not confined to contract data"),
    );
    let repository = directory.display().to_string();
    for args in [
        vec!["pinned", "--classify-only"],
        vec![
            "pinned",
            "--classify-only",
            "--accept-base-analyzer",
            "HEAD",
        ],
        vec![
            "pinned",
            "--classify-only",
            "--classify-only",
            "--accept-base-analyzer",
            "HEAD",
            "--repository",
            &repository,
        ],
        vec![
            "pinned",
            "--classify-only",
            "--checkpoint-dir",
            "never-created",
            "--accept-base-analyzer",
            "HEAD",
            "--repository",
            &repository,
        ],
        vec![
            "pinned",
            "--classify-only",
            "--accept-base-analyzer",
            "HEAD",
            "--repository",
            &repository,
            "/nonexistent-root",
        ],
    ] {
        let output = Command::new(HOST).args(&args).output().unwrap();
        assert!(!output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
    }
    assert!(!std::path::Path::new("never-created").exists());
    std::fs::remove_dir_all(&directory).unwrap();
}
