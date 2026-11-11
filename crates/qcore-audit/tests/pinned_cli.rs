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
    ] {
        let output = Command::new(HOST).args(&args).output().unwrap();
        assert!(!output.status.success(), "{args:?}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!error.contains("could not resolve"), "{args:?}: {error}");
    }
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
