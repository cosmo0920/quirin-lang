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

//! Card W45-CONTRACT-REVISION-MODE, end to end on the command line:
//! `quirin-audit pin` writes and checks a lock, `quirin-audit contracts`
//! evaluates it (a pinned contract is in force only where the audited source
//! matches), and `quirin-audit compare` reports the change between two runs.
//!
//! The fixture is the generic held-out mutation fixture
//! `tests/fixtures/revision_mode/` (derived from no production source). The
//! second test replays its whole mutation table, the one the report's harness
//! (`docs/reports/w45-contract-revision-mode/scripts/mutations.py`) publishes
//! with per-mutation logs, so the table is part of the gate.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

static NEXT: AtomicUsize = AtomicUsize::new(0);

const FLEXIBLE_MARKER: &str = "    requires resource_revision(flexible);\n";

fn fixture(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/revision_mode")
            .join(name),
    )
    .unwrap()
    .replace("\r\n", "\n")
}

fn clang_available() -> bool {
    if qcore_c_gateway::clang_command().is_ok() {
        return true;
    }
    assert!(
        std::env::var_os("QCORE_CLANG").is_none(),
        "explicit QCORE_CLANG is invalid"
    );
    eprintln!("skipping Clang-dependent revision-mode audit test");
    false
}

struct Tree(PathBuf);

impl Tree {
    /// A fresh tree: `src/queue.c` (the fixture), `contracts/queue.qv`
    /// (pinned or flexible) and `manifest.json`.
    fn new(pinned: bool) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-revision-mode-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("contracts")).unwrap();
        fs::write(root.join("src/queue.c"), fixture("queue.c")).unwrap();
        let contracts = fixture("queue.qv");
        fs::write(
            root.join("contracts/queue.qv"),
            if pinned {
                contracts.replace(FLEXIBLE_MARKER, "")
            } else {
                contracts
            },
        )
        .unwrap();
        fs::write(
            root.join("manifest.json"),
            "{\"schema_version\": 1, \"modules\": [\"contracts/queue.qv\"]}\n",
        )
        .unwrap();
        Self(root)
    }

    fn quirin(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }

    fn pin(&self, extra: &[&str]) -> Output {
        let mut args = vec!["pin", "--source-root", "src", "--modules", "contracts"];
        args.extend_from_slice(extra);
        self.quirin(&args)
    }

    fn audit(&self, report: &str) -> Output {
        self.quirin(&[
            "contracts",
            "--source-root",
            "src",
            "--contract-manifest",
            "manifest.json",
            "--report",
            report,
        ])
    }

    fn read(&self, path: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(path)).unwrap()).unwrap()
    }

    fn mutate(&self, edits: &[(String, String)]) {
        let path = self.0.join("src/queue.c");
        let mut text = fs::read_to_string(&path).unwrap();
        for (old, new) in edits {
            assert_eq!(text.matches(old.as_str()).count(), 1, "{old}");
            text = text.replacen(old.as_str(), new, 1);
        }
        fs::write(path, text).unwrap();
    }

    fn statuses(&self, report: &str) -> Vec<(String, String, Vec<String>)> {
        let report = self.read(report);
        report["units"][0]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|function| {
                let reasons = function["diagnostics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|d| {
                        let description = d["description"].as_str().unwrap();
                        if description.ends_with("(contract-pin-mismatch)") {
                            "contract-pin-mismatch".to_owned()
                        } else {
                            d["category"].as_str().unwrap().to_owned()
                        }
                    })
                    .collect();
                (
                    function["function"].as_str().unwrap().to_owned(),
                    function["status"].as_str().unwrap().to_owned(),
                    reasons,
                )
            })
            .collect()
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn succeeded(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn edits(mutation: &Value) -> Vec<(String, String)> {
    mutation["edits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| {
            (
                pair[0].as_str().unwrap().to_owned(),
                pair[1].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn mutation(name: &str) -> Value {
    let spec: Value = serde_json::from_str(&fixture("mutations.json")).unwrap();
    spec["mutations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["name"] == name)
        .unwrap()
        .clone()
}

#[test]
fn a_pin_holds_until_the_source_changes_and_the_change_report_says_why() {
    if !clang_available() {
        return;
    }
    let tree = Tree::new(true);
    // Without a lock every pinned contract is out of force (no-lock).
    succeeded(&tree.audit("nolock/r.json"));
    let section = tree.read("nolock/r.contracts.json")["contract_revision"].clone();
    assert_eq!(section["counts"]["pinned"], 10);
    assert_eq!(section["counts"]["pinned_not_in_force"], 10);
    assert!(
        section["pin_mismatches"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["reason"] == "no-lock")
    );
    // Pin, then check: up to date.
    succeeded(&tree.pin(&[]));
    let lock = tree.read("contracts/contracts.lock");
    assert_eq!(lock["owner"], "contracts");
    // Card CONTRACT-SUPERSESSION: version 4, every entry stamped.
    assert_eq!(lock["lock_version"], 4);
    assert!(lock["pinned"][0]["pinned_at"].is_string());
    assert_eq!(
        lock["pinned"][0]["identities"][0]["reviewed_at"],
        serde_json::json!([{"kind": "directory"}])
    );
    assert_eq!(lock["pinned"].as_array().unwrap().len(), 10);
    assert_eq!(lock["flexible"], serde_json::json!([]));
    succeeded(&tree.pin(&["--check"]));
    // In force: every function Safe.
    succeeded(&tree.audit("before/r.json"));
    assert!(
        tree.statuses("before/r.json")
            .iter()
            .all(|(_, status, _)| status == "Safe")
    );
    let before = tree.read("before/r.contracts.json")["contract_revision"].clone();
    assert_eq!(before["counts"]["pinned_in_force"], 10);
    assert_eq!(before["contracts"]["box_drop"]["status"], "pinned-in-force");
    // A body change: `box_drop` is out of force and its caller is held (card
    // W45-USER-CONTRACT-PLUGINS made it Pending; card
    // W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL keeps it Unknown, below).
    tree.mutate(&edits(&mutation("dropped-release")));
    let check = tree.pin(&["--check"]);
    assert_eq!(check.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&check.stdout)
            .contains("x box_drop: not in force (source-changed)")
    );
    succeeded(&tree.audit("after/r.json"));
    let after = tree.read("after/r.contracts.json")["contract_revision"].clone();
    assert_eq!(after["counts"]["pinned_not_in_force"], 1);
    let mismatch = &after["pin_mismatches"][0];
    assert_eq!(mismatch["function"], "box_drop");
    assert_eq!(mismatch["reason"], "source-changed");
    assert_eq!(mismatch["held_call_sites"], 1);
    let statuses = tree.statuses("after/r.json");
    let user_drop = statuses
        .iter()
        .find(|(name, ..)| name == "user_drop")
        .unwrap();
    // Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL: `Unknown`, not
    // `Pending`. The re-pin below shows why: with the declaration back in
    // force its body conflicts with it, so no re-pin alone makes `user_drop`
    // `Safe`, and that standing reason is recorded beside the held call.
    assert_eq!(user_drop.1, "Unknown");
    assert_eq!(user_drop.2, ["contract-pin-mismatch", "unsupported"]);
    assert_eq!(
        tree.read("after/r.json")["units"][0]["reason_code"],
        "contract-pin-mismatch"
    );
    // The change report separates lost coverage from detected violations.
    let compare = tree.quirin(&[
        "compare",
        "--before",
        "before/r.json",
        "--after",
        "after/r.json",
        "--output",
        "compare.json",
    ]);
    succeeded(&compare);
    let report = tree.read("compare.json");
    assert_eq!(
        report["pin_mismatches"]["new"],
        serde_json::json!(["box_drop"])
    );
    assert_eq!(report["counts"]["violation_detected"], 0);
    assert_eq!(report["counts"]["coverage_lost"], 1);
    let lost = &report["verdict_moves"]["coverage_lost"][0];
    assert_eq!(lost["function"], "user_drop");
    assert_eq!(
        lost["reasons"],
        serde_json::json!(["contract-pin-mismatch", "unsupported"])
    );
    assert_eq!(
        report["contracts_in_force"]["removed"][0]["function"],
        "box_drop"
    );
    assert!(
        report["obligations"]["added"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["function"] == "user_drop")
    );
    // Re-pinning the changed body after review puts the contract back in
    // force, where it is checked against that body as usual: the dropped
    // release is now a conflict of the body with its contract, not a pin.
    succeeded(&tree.pin(&["--function", "box_drop"]));
    succeeded(&tree.audit("repinned/r.json"));
    assert_eq!(
        tree.read("repinned/r.contracts.json")["contract_revision"]["counts"]["pinned_not_in_force"],
        0
    );
    let repinned = tree.statuses("repinned/r.json");
    let box_drop = repinned
        .iter()
        .find(|(name, ..)| name == "box_drop")
        .unwrap();
    assert_eq!(box_drop.1, "Unknown");
    assert_eq!(box_drop.2, ["summary_conflict"]);
    // Restoring the reviewed body and its pin restores every verdict exactly.
    fs::write(tree.0.join("src/queue.c"), fixture("queue.c")).unwrap();
    succeeded(&tree.pin(&["--function", "box_drop"]));
    succeeded(&tree.pin(&["--check"]));
    succeeded(&tree.audit("restored/r.json"));
    assert_eq!(
        tree.statuses("restored/r.json"),
        tree.statuses("before/r.json")
    );
    assert_eq!(tree.read("restored/r.json"), tree.read("before/r.json"));
    // A changed contract text is out of force until it is pinned again.
    let path = tree.0.join("contracts/queue.qv");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replacen(
            "resource_released(item, nonnull)",
            "resource_released(item, nullable)",
            1,
        ),
    )
    .unwrap();
    succeeded(&tree.audit("text/r.json"));
    let text_section = tree.read("text/r.contracts.json")["contract_revision"].clone();
    assert_eq!(
        text_section["pin_mismatches"][0]["reason"],
        "contract-changed"
    );
}

#[test]
fn a_malformed_lock_refuses_the_audit() {
    let tree = Tree::new(true);
    fs::write(
        tree.0.join("contracts/contracts.lock"),
        "{\"lock_version\": 1}",
    )
    .unwrap();
    let output = tree.audit("r.json");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("malformed contract lock"));
    // Adding an identity needs a lock it can read; only a regeneration
    // replaces an unreadable one, and says so.
    assert_eq!(tree.pin(&[]).status.code(), Some(2));
    let regenerated = tree.pin(&["--regenerate"]);
    succeeded(&regenerated);
    assert!(String::from_utf8_lossy(&regenerated.stdout).contains("replaces an unreadable lock"));
    let path = tree.0.join("contracts/contracts.lock");
    let mut lock: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    lock["pinned"].as_array_mut().unwrap().reverse();
    fs::write(&path, serde_json::to_string(&lock).unwrap()).unwrap();
    let output = tree.audit("r.json");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not sorted and unique"));
}

/// One expected row: mutation, pinned, and the watched functions' statuses.
type Row = (&'static str, bool, &'static [(&'static str, &'static str)]);

/// The controlled-mutation table (criterion 5 of the card): under flexible
/// contracts a harmless refactor stays accepted and each defect is caught or
/// reported as lost coverage, never accepted; under pinned contracts every
/// body change puts the mutated function's contract out of force.
#[test]
fn the_mutation_table() {
    if !clang_available() {
        return;
    }
    // (mutation, mode) -> the mutated function's and its callers' statuses.
    let expected: &[Row] = &[
        ("rename-local", false, &[("box_count", "Safe")]),
        ("reorder-independent", false, &[("box_two", "Safe")]),
        (
            "dropped-release",
            false,
            &[("box_drop", "Unknown"), ("user_drop", "Unknown")],
        ),
        (
            "missing-counter-rollback",
            false,
            &[("box_count", "Unsafe")],
        ),
        (
            "changed-transfer",
            false,
            &[("box_keep", "Unknown"), ("user_keep", "Unknown")],
        ),
        (
            "premature-destroy",
            false,
            &[("box_drop", "Unknown"), ("user_drop", "Safe")],
        ),
        // Card W45-USER-CONTRACT-PLUGINS: a caller whose only unresolved
        // diagnostic is the held call is `Pending` (a re-pin after review
        // resolves it); it was `Unknown`. Card
        // W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL: only where the re-pin
        // as written would leave it `Safe`, which is the flexible row above
        // (`premature-destroy`); `dropped-release` and `changed-transfer`
        // leave the re-pinned declaration refuted by its own body, so their
        // callers are `Unknown` with that reason standing beside the held
        // call (they were `Pending`: corrections).
        ("rename-local", true, &[("box_count", "Safe")]),
        ("reorder-independent", true, &[("box_two", "Safe")]),
        (
            "dropped-release",
            true,
            &[("box_drop", "Safe"), ("user_drop", "Unknown")],
        ),
        ("missing-counter-rollback", true, &[("box_count", "Unsafe")]),
        (
            "changed-transfer",
            true,
            &[("box_keep", "Safe"), ("user_keep", "Unknown")],
        ),
        (
            "premature-destroy",
            true,
            &[("box_drop", "Unknown"), ("user_drop", "Pending")],
        ),
    ];
    for (name, pinned, outcome) in expected {
        let tree = Tree::new(*pinned);
        if *pinned {
            succeeded(&tree.pin(&[]));
        }
        let spec = mutation(name);
        tree.mutate(&edits(&spec));
        succeeded(&tree.audit("r.json"));
        let statuses = tree.statuses("r.json");
        for (function, status) in *outcome {
            let found = statuses.iter().find(|(n, ..)| n == function).unwrap();
            assert_eq!(
                found.1, *status,
                "{name} pinned={pinned} {function}: {found:?}"
            );
            if *pinned && function.starts_with("user_") {
                // The held call is always named; a `Pending` row carries
                // nothing else, an `Unknown` one its standing reasons.
                if *status == "Pending" {
                    assert_eq!(found.2, ["contract-pin-mismatch"], "{name}");
                } else {
                    assert!(
                        found.2.iter().any(|r| r == "contract-pin-mismatch"),
                        "{name}"
                    );
                    assert!(
                        found
                            .2
                            .iter()
                            .all(|r| r == "contract-pin-mismatch" || r == "unsupported"),
                        "{name}: {found:?}"
                    );
                }
            }
        }
        let section = tree.read("r.contracts.json")["contract_revision"].clone();
        let withheld = section["pin_mismatches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["function"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        if *pinned {
            // Every body change puts the mutated function's contract out of
            // force, and only that one.
            assert_eq!(withheld, [spec["function"].as_str().unwrap()], "{name}");
        } else {
            assert!(withheld.is_empty(), "{name}");
            // A harmless refactor stays accepted; a defect is never accepted.
            let mutated = outcome[0].1;
            if spec["kind"] == "harmless" {
                assert_eq!(mutated, "Safe", "{name}");
            } else {
                assert!(outcome.iter().any(|(_, s)| *s != "Safe"), "{name}");
            }
        }
    }
}

#[test]
fn qv_directory_discovery_and_lock_evaluation_preserve_checked_contracts() {
    if !clang_available() {
        return;
    }
    let legacy = Tree::new(true);
    succeeded(&legacy.pin(&[]));
    succeeded(&legacy.audit("old/r.json"));
    let current = Tree::new(true);
    fs::rename(
        current.0.join("contracts/queue.qv"),
        current.0.join("contracts/queue.qv"),
    )
    .unwrap();
    fs::write(
        current.0.join("manifest.json"),
        "{\"schema_version\":1,\"modules\":[\"contracts/queue.qv\"]}\n",
    )
    .unwrap();
    succeeded(&current.pin(&[]));
    succeeded(&current.pin(&["--check"]));
    succeeded(&current.audit("new/r.json"));
    assert_eq!(
        legacy.statuses("old/r.json"),
        current.statuses("new/r.json")
    );
    assert_eq!(
        legacy.read("old/r.contracts.json")["contract_revision"]["counts"],
        current.read("new/r.contracts.json")["contract_revision"]["counts"]
    );
    fs::write(current.0.join("contracts/queue.qv"), fixture("queue.qv")).unwrap();
    assert!(
        !current.pin(&["--check"]).status.success(),
        "two aliases must not be picked by directory order"
    );
}
