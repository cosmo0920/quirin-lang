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

//! Card CONTRACT-PIN-TIP-MODE, end to end on the command line: a lock entry
//! that tracks the tip of a branch (lock format version 3).
//!
//! On a real Git history of the generic fixture `tests/fixtures/revision_mode/`
//! (derived from no production source), whose checkout is both the audited
//! source and the tracked repository (branch `fix`):
//!
//! * the tip resolves and matches: in force, an `info` notice on `pin`,
//!   `pin --check`, the audit's tally and the sidecar, the resolved commit
//!   recorded, a named assumption, inventory status `tip-in-force`;
//! * verdicts are byte-identical to the same contract pinned by a reviewed
//!   identity at the same sites, and an unresolvable tip's to a missing entry:
//!   the notice and the tip path decide nothing;
//! * the tip moves with the audited source: still in force, the notice says
//!   the commit was not reviewed; the tip moves away from it: `source-changed`
//!   with a `warning`, the caller held;
//! * repository not located, ref missing, diverged branch: `tip-unresolved`
//!   with a `warning`; a malformed repository variable refuses;
//! * a reviewed pin beside the tip entry is unaffected throughout;
//! * `compare` reports the tip that moved;
//! * the lock: written as version 3 only with a tip entry, kept by other pin
//!   runs and `--regenerate`, turned back by `--untrack-tip`; a version-2 lock
//!   with a tip entry, a version-3 lock without one and version 4 refuse.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

static NEXT: AtomicUsize = AtomicUsize::new(0);

const FLEXIBLE_MARKER: &str = "    requires resource_revision(flexible);\n";
const VARIABLE: &str = "QUIRIN_TIP_REPOSITORIES";
const TARGET: &str = "targets a future, unreleased release";

const BOX_DROP_A: &str = "    item_touch(item);\n    item_free(item);\n}";
const BOX_DROP_B: &str = "    item_touch(item);\n    item_touch(item);\n    item_free(item);\n}";

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
    eprintln!("skipping Clang-dependent tip-mode test");
    false
}

struct Tree(PathBuf);

impl Tree {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-tip-mode-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("contracts")).unwrap();
        fs::write(root.join("src/queue.c"), fixture("queue.c")).unwrap();
        fs::write(
            root.join("contracts/queue.qv"),
            fixture("queue.qv").replace(FLEXIBLE_MARKER, ""),
        )
        .unwrap();
        fs::write(
            root.join("manifest.json"),
            "{\"schema_version\": 1, \"modules\": [\"contracts/queue.qv\"]}\n",
        )
        .unwrap();
        let tree = Self(root);
        tree.vcs(&["init", "-q"]);
        tree
    }

    fn quirin_with(&self, args: &[&str], variable: Option<&str>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
        command.current_dir(&self.0).args(args).env_remove(VARIABLE);
        if let Some(value) = variable {
            command.env(VARIABLE, value);
        }
        command.output().unwrap()
    }

    fn quirin(&self, args: &[&str]) -> Output {
        self.quirin_with(args, None)
    }

    fn pin(&self, extra: &[&str]) -> Output {
        let mut args = vec!["pin", "--source-root", "src", "--modules", "contracts"];
        args.extend_from_slice(extra);
        self.quirin(&args)
    }

    /// Audit `src`; the sidecar's `contract_revision` section and stdout.
    fn audit_with(&self, report: &str, variable: Option<&str>) -> (Value, String) {
        let output = self.quirin_with(
            &[
                "contracts",
                "--source-root",
                "src",
                "--contract-manifest",
                "manifest.json",
                "--report",
                report,
            ],
            variable,
        );
        let stdout = succeeded(&output);
        (
            self.read(&report.replace(".json", ".contracts.json"))["contract_revision"].clone(),
            stdout,
        )
    }

    fn audit(&self, report: &str) -> (Value, String) {
        self.audit_with(report, None)
    }

    fn read(&self, path: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(path)).unwrap()).unwrap()
    }

    fn units(&self, report: &str) -> Value {
        self.read(report)["units"].clone()
    }

    fn status(&self, report: &str, function: &str) -> (String, Vec<String>) {
        let report = self.read(report);
        let row = report["units"][0]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["function"] == function)
            .unwrap()
            .clone();
        let reasons = row["diagnostics"]
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
        (row["status"].as_str().unwrap().to_owned(), reasons)
    }

    fn replace(&self, old: &str, new: &str) {
        let path = self.0.join("src/queue.c");
        let text = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
        assert_eq!(text.matches(old).count(), 1, "{old}");
        fs::write(path, text.replacen(old, new, 1)).unwrap();
    }

    fn vcs(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(self.0.join("src"))
            .args([
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{args:?}: {output:?}");
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn commit(&self, message: &str) -> String {
        self.vcs(&["add", "-A"]);
        self.vcs(&["commit", "-q", "-m", message]);
        self.vcs(&["rev-parse", "HEAD"])
    }

    fn lock_path(&self) -> PathBuf {
        self.0.join("contracts/contracts.lock")
    }

    fn lock(&self) -> Value {
        self.read("contracts/contracts.lock")
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn succeeded(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    stdout
}

fn record<'a>(section: &'a Value, list: &str, function: &str) -> &'a Value {
    section[list]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["function"] == function)
        .unwrap_or_else(|| panic!("{function} not in {list}: {section}"))
}

fn notice_lines(stdout: &str, severity: &str) -> usize {
    stdout
        .lines()
        .filter(|line| line.contains(&format!("notice [{severity}]")) && line.contains(TARGET))
        .count()
}

/// A tree at commit A with every contract pinned and `box_drop` tracking the
/// tip of `fix` (at A). Returns the tree, A and the reviewed-only lock text.
fn tracked_tree() -> (Tree, String, String) {
    let tree = Tree::new();
    let a = tree.commit("A");
    tree.vcs(&["branch", "fix"]);
    succeeded(&tree.pin(&[]));
    let reviewed = fs::read_to_string(tree.lock_path()).unwrap();
    let tracked = tree.pin(&[
        "--function",
        "box_drop",
        "--track-tip",
        "fix",
        "--release-label",
        "queue next release",
    ]);
    let stdout = succeeded(&tracked);
    assert!(
        stdout.contains("~ box_drop: now tracks the tip `refs/heads/fix`"),
        "{stdout}"
    );
    assert_eq!(notice_lines(&stdout, "info"), 1, "{stdout}");
    assert!(stdout.contains("queue next release"), "{stdout}");
    (tree, a, reviewed)
}

#[test]
fn a_tip_entry_is_in_force_at_its_tip_and_held_elsewhere() {
    if !clang_available() {
        return;
    }
    let (tree, a, reviewed) = tracked_tree();
    let lock = tree.lock();
    // Card CONTRACT-SUPERSESSION: every lock is written in version 4.
    assert_eq!(lock["lock_version"], 4);
    assert!(
        lock["pinned"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["function"] != "box_drop")
    );
    let entry = &lock["tip"][0];
    assert_eq!(entry["function"], "box_drop");
    assert_eq!(entry["ref"], "refs/heads/fix");
    assert_eq!(entry["reviewed"]["commit"], a.as_str());
    assert_eq!(entry["release_label"], "queue next release");

    // In force at the tip: info everywhere, the resolved commit recorded, a
    // named assumption, never `pinned-in-force`; the reviewed pin beside it
    // is unaffected.
    let (section, stdout) = tree.audit("at-tip/r.json");
    let in_force = record(&section, "pinned_in_force", "box_drop");
    assert_eq!(in_force["matched_tip_commit"], a.as_str());
    assert_eq!(in_force["tip"]["resolved_commit"], a.as_str());
    assert_eq!(in_force["tip"]["moved_since_review"], false);
    assert_eq!(in_force["notice"]["severity"], "info");
    assert_eq!(section["tip_notices"][0]["severity"], "info");
    assert_eq!(section["tip_notices"][0]["function"], "box_drop");
    assert_eq!(section["counts"]["tip_tracked"], 1);
    assert_eq!(section["counts"]["tip_in_force"], 1);
    assert_eq!(
        section["assumptions"]["tip_tracked"][0]["name"],
        "tip-tracked:box_drop"
    );
    assert_eq!(section["contracts"]["box_drop"]["status"], "tip-in-force");
    assert_eq!(
        section["contracts"]["item_free"]["status"],
        "pinned-in-force"
    );
    assert_eq!(
        record(&section, "pinned_in_force", "item_free")["matched_reviewed_at"][0]["revision"],
        a.as_str()
    );
    assert!(
        stdout.contains("1 tracking the tip of an unreleased branch (1 in force, 0 not in force)")
    );
    assert_eq!(notice_lines(&stdout, "info"), 1, "{stdout}");
    assert_eq!(tree.status("at-tip/r.json", "user_drop").0, "Safe");
    let check = tree.pin(&["--check"]);
    let check_out = succeeded(&check);
    assert!(check_out.contains("t box_drop: in force through the tip `refs/heads/fix`"));
    assert_eq!(notice_lines(&check_out, "info"), 1, "{check_out}");

    // The same contract pinned by a reviewed identity at the same sites gives
    // byte-identical verdicts: the tip path and its notice decide nothing.
    let tip_lock = fs::read(tree.lock_path()).unwrap();
    fs::write(tree.lock_path(), &reviewed).unwrap();
    let (reviewed_section, reviewed_out) = tree.audit("reviewed/r.json");
    assert!(reviewed_section.get("tip_notices").is_none());
    assert!(reviewed_section["counts"].get("tip_tracked").is_none());
    assert_eq!(notice_lines(&reviewed_out, "info"), 0);
    assert_eq!(tree.units("reviewed/r.json"), tree.units("at-tip/r.json"));
    fs::write(tree.lock_path(), &tip_lock).unwrap();

    // The tip moves with the audited source (HEAD on `fix`): still in force,
    // and the notice says the new commit was not reviewed.
    tree.vcs(&["checkout", "-q", "fix"]);
    tree.replace(BOX_DROP_A, BOX_DROP_B);
    let b = tree.commit("B");
    let (section, _) = tree.audit("moved/r.json");
    let in_force = record(&section, "pinned_in_force", "box_drop");
    assert_eq!(in_force["tip"]["resolved_commit"], b.as_str());
    assert_eq!(in_force["tip"]["moved_since_review"], true);
    assert_eq!(in_force["tip"]["sites_changed_since_review"], true);
    assert!(
        in_force["notice"]["message"]
            .as_str()
            .unwrap()
            .contains("was not reviewed")
    );
    assert_eq!(
        section["assumptions"]["tip_tracked"][0]["moved_since_review"],
        true
    );

    // The audited source is A, the tip B: held under `source-changed`, a
    // warning on every surface, the caller held, the reviewed pin in force.
    tree.vcs(&["checkout", "-q", &a]);
    let (section, stdout) = tree.audit("at-a/r.json");
    let held = record(&section, "pin_mismatches", "box_drop");
    assert_eq!(held["reason"], "source-changed");
    assert!(held["detail"].as_str().unwrap().contains("refs/heads/fix"));
    assert_eq!(held["tip"]["resolved_commit"], b.as_str());
    assert_eq!(held["notice"]["severity"], "warning");
    assert!(
        held["notice"]["message"]
            .as_str()
            .unwrap()
            .contains("NOT in force for the audited source")
    );
    assert_eq!(section["tip_notices"][0]["severity"], "warning");
    assert!(
        section["assumptions"]["tip_tracked"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(section["contracts"]["box_drop"]["status"], "not-in-force");
    assert_eq!(
        section["contracts"]["item_free"]["status"],
        "pinned-in-force"
    );
    assert_eq!(notice_lines(&stdout, "warning"), 1, "{stdout}");
    let (status, reasons) = tree.status("at-a/r.json", "user_drop");
    assert_ne!(status, "Safe");
    assert!(
        reasons.contains(&"contract-pin-mismatch".to_owned()),
        "{reasons:?}"
    );
    let check = tree.pin(&["--check"]);
    assert_eq!(check.status.code(), Some(1));
    let check_out = String::from_utf8_lossy(&check.stdout);
    assert!(check_out.contains("x box_drop: not in force (source-changed)"));
    assert_eq!(notice_lines(&check_out, "warning"), 1, "{check_out}");

    // `compare` names the tip that moved and prints both runs' notices.
    let compare = tree.quirin(&[
        "compare",
        "--before",
        "at-tip/r.json",
        "--after",
        "at-a/r.json",
        "--output",
        "compare.json",
    ]);
    let compare_out = succeeded(&compare);
    assert!(compare_out.contains("tip-tracked contracts 1 -> 1 (1 tip(s) moved)"));
    assert!(compare_out.contains("compare notice (after) [warning]"));
    let report = tree.read("compare.json");
    assert_eq!(report["tip_tracking"]["moved"][0]["before"], a.as_str());
    assert_eq!(report["tip_tracking"]["moved"][0]["after"], b.as_str());
    assert_eq!(report["tip_tracking"]["after"][0]["in_force"], false);

    // Ref missing: `tip-unresolved`, a warning.
    tree.vcs(&["branch", "-m", "fix", "fix-renamed"]);
    let (section, stdout) = tree.audit("no-ref/r.json");
    let held = record(&section, "pin_mismatches", "box_drop");
    assert_eq!(held["reason"], "tip-unresolved");
    assert_eq!(held["tip"]["failure"], "ref-missing");
    assert_eq!(notice_lines(&stdout, "warning"), 1);
    // An unresolvable tip decides exactly as a missing entry.
    let mut without = tree.lock();
    without["tip"] = serde_json::json!([]);
    without["lock_version"] = serde_json::json!(4);
    let tip_lock = fs::read(tree.lock_path()).unwrap();
    fs::write(
        tree.lock_path(),
        serde_json::to_string_pretty(&without).unwrap(),
    )
    .unwrap();
    let (section, _) = tree.audit("no-entry/r.json");
    assert_eq!(
        record(&section, "pin_mismatches", "box_drop")["reason"],
        "not-in-lock"
    );
    assert_eq!(tree.units("no-entry/r.json"), tree.units("no-ref/r.json"));
    fs::write(tree.lock_path(), &tip_lock).unwrap();
    tree.vcs(&["branch", "-m", "fix-renamed", "fix"]);

    // Diverged: `fix` rewritten to a history without the reviewed commit.
    tree.vcs(&["checkout", "-q", "--orphan", "other"]);
    let orphan = tree.commit("orphan");
    tree.vcs(&["branch", "-f", "fix", &orphan]);
    tree.vcs(&["checkout", "-q", &a]);
    let (section, _) = tree.audit("diverged/r.json");
    let held = record(&section, "pin_mismatches", "box_drop");
    assert_eq!(held["reason"], "tip-unresolved");
    assert_eq!(held["tip"]["failure"], "diverged");
    tree.vcs(&["branch", "-f", "fix", &b]);

    // Repository not located: the variable points the name elsewhere.
    let elsewhere = format!("src={}", tree.0.join("contracts").display());
    let (section, _) = tree.audit_with("elsewhere/r.json", Some(&elsewhere));
    let held = record(&section, "pin_mismatches", "box_drop");
    assert_eq!(held["tip"]["failure"], "repository-missing");
    assert_eq!(held["tip"]["located_by"], "environment");
    // And through the variable to the real repository, in force again at B.
    tree.vcs(&["checkout", "-q", "fix"]);
    let here = format!(
        "src={}",
        tree.0.join("src").canonicalize().unwrap().display()
    );
    let (section, _) = tree.audit_with("located/r.json", Some(&here));
    assert_eq!(
        record(&section, "pinned_in_force", "box_drop")["tip"]["located_by"],
        "environment"
    );
    // A malformed variable refuses the audit.
    let refused = tree.quirin_with(
        &[
            "contracts",
            "--source-root",
            "src",
            "--contract-manifest",
            "manifest.json",
            "--report",
            "refused/r.json",
        ],
        Some("src=relative/path"),
    );
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains(VARIABLE));
}

#[test]
fn a_tip_entry_keeps_its_payload_and_version_four_review_stamps() {
    let (tree, _a, reviewed) = tracked_tree();
    // Every other pin run keeps the tip entry and prints its notice.
    let tip_lock = fs::read_to_string(tree.lock_path()).unwrap();
    let again = succeeded(&tree.pin(&[]));
    assert_eq!(fs::read_to_string(tree.lock_path()).unwrap(), tip_lock);
    assert_eq!(notice_lines(&again, "info"), 1, "{again}");
    succeeded(&tree.pin(&["--regenerate"]));
    // Version 4 refreshes review stamps on regenerated reviewed entries; the
    // source identities and the independently retained tip must remain exact.
    let regenerated = fs::read_to_string(tree.lock_path()).unwrap();
    let old_tip: serde_json::Value = serde_json::from_str(&tip_lock).unwrap();
    let new_tip: serde_json::Value = serde_json::from_str(&regenerated).unwrap();
    assert_eq!(
        old_tip["tip"], new_tip["tip"],
        "regeneration changed the retained tip"
    );
    assert_same_pin_payload_with_monotone_stamps(&tip_lock, &regenerated);
    // `--untrack-tip` turns it back into a reviewed identity at HEAD (A):
    // the lock is again the reviewed-only lock (no `tip` section), equal to
    // it except for review stamps, which may only move forward.
    succeeded(&tree.pin(&["--function", "box_drop", "--untrack-tip"]));
    assert_same_pin_payload_with_monotone_stamps(
        &reviewed,
        &fs::read_to_string(tree.lock_path()).unwrap(),
    );
    let refused = tree.pin(&["--function", "box_drop", "--untrack-tip"]);
    assert_eq!(refused.status.code(), Some(2));
    // A flexible or unknown function, or a missing ref, cannot be tracked.
    let missing = tree.pin(&["--function", "box_drop", "--track-tip", "no-such-branch"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("names no commit"));
    // Malformed versions.
    fs::write(tree.lock_path(), &tip_lock).unwrap();
    let lock: Value = serde_json::from_str(&tip_lock).unwrap();
    let mut two = lock.clone();
    two["lock_version"] = serde_json::json!(2);
    let mut three = serde_json::from_str::<Value>(&reviewed).unwrap();
    three["lock_version"] = serde_json::json!(3);
    // Card CONTRACT-SUPERSESSION: version 4 is the stamped format every
    // lock is written in, so the unknown version is 5.
    let mut four = lock.clone();
    four["lock_version"] = serde_json::json!(5);
    let mut relative = lock;
    relative["tip"][0]["repository"]["path"] = serde_json::json!("relative/src");
    for (bad, expected) in [
        (two, "version-2 lock holds a tip entry"),
        (three, "version-3 lock holds no tip entry"),
        (four, "unsupported lock_version"),
        (relative, "repository path is not absolute"),
    ] {
        fs::write(tree.lock_path(), serde_json::to_string(&bad).unwrap()).unwrap();
        let output = tree.pin(&["--check"]);
        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected), "{stderr}");
    }
}

fn assert_same_pin_payload_with_monotone_stamps(before: &str, after: &str) {
    let before: serde_json::Value = serde_json::from_str(before).unwrap();
    let mut after: serde_json::Value = serde_json::from_str(after).unwrap();
    assert_eq!(before["lock_version"], 4);
    assert_eq!(after["lock_version"], 4);
    for section in ["pinned", "flexible", "tip"] {
        // Card LANDING-REPAIR: the lock omits an empty `tip` section
        // (`ContractLock::tip` is `skip_serializing_if = "Vec::is_empty"`
        // since CONTRACT-PIN-TIP-MODE), so a reviewed-only lock has none.
        // Both locks must agree on whether a section is present; an absent
        // section holds no entry, and every present entry needs its stamp.
        assert_eq!(
            before.get(section).is_some(),
            after.get(section).is_some(),
            "section `{section}` appeared or disappeared"
        );
        let Some(old) = before.get(section) else {
            continue;
        };
        let old = old.as_array().unwrap();
        let new = after[section].as_array_mut().unwrap();
        assert_eq!(old.len(), new.len());
        for (a, b) in old.iter().zip(new) {
            let old_stamp = a["pinned_at"]
                .as_str()
                .expect("an entry without a pin stamp");
            let new_stamp = b["pinned_at"]
                .as_str()
                .expect("an entry without a pin stamp");
            assert!(new_stamp >= old_stamp, "review stamp moved backwards");
            b["pinned_at"] = a["pinned_at"].clone();
        }
    }
    assert_eq!(
        before, after,
        "a pin payload changed beyond its review stamp"
    );
}
