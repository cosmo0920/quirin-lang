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

//! Card W45-CONTRACT-REVISION-MODE, integration review: a pinned lock entry
//! holds several **reviewed identities**, each the function's sites at the
//! revisions it was reviewed at (lock format version 2).
//!
//! On a real Git history of the generic fixture
//! `tests/fixtures/revision_mode/`: an entry pinned at revisions A and B is
//! in force at both and out of force at C, whose body differs from both;
//! adding B does not disturb A; a contract edit voids every identity; an
//! identity is dropped by revision; `--regenerate` starts from one revision;
//! a version-1 lock is still read.

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
    eprintln!("skipping Clang-dependent revision-identity test");
    false
}

/// A fresh tree: `src/queue.c` (the fixture), `contracts/queue.qv` with
/// every contract pinned (the default), and `manifest.json`.
struct Tree(PathBuf);

impl Tree {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-revision-identities-{}-{}",
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
        Self(root)
    }

    fn src(&self) -> PathBuf {
        self.0.join("src")
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

    fn audit(&self, report: &str) -> Value {
        succeeded(&self.quirin(&[
            "contracts",
            "--source-root",
            "src",
            "--contract-manifest",
            "manifest.json",
            "--report",
            report,
        ]));
        self.read(&report.replace(".json", ".contracts.json"))["contract_revision"].clone()
    }

    fn read(&self, path: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(path)).unwrap()).unwrap()
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
        let path = self.src().join("queue.c");
        let text = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
        assert_eq!(text.matches(old).count(), 1, "{old}");
        fs::write(path, text.replacen(old, new, 1)).unwrap();
    }

    fn vcs(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(self.src())
            .args([
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
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

    fn entry(&self, function: &str) -> Value {
        self.read("contracts/contracts.lock")["pinned"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["function"] == function)
            .unwrap()
            .clone()
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

fn mismatches(section: &Value) -> Vec<(String, String)> {
    section["pin_mismatches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                m["function"].as_str().unwrap().to_owned(),
                m["reason"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

const BOX_DROP_A: &str = "    item_touch(item);\n    item_free(item);\n}";
const BOX_DROP_B: &str = "    item_touch(item);\n    item_touch(item);\n    item_free(item);\n}";
const BOX_DROP_C: &str = "    item_touch(item);\n}";

#[test]
fn a_pin_holds_at_every_reviewed_revision_and_nowhere_else() {
    if !clang_available() {
        return;
    }
    let tree = Tree::new();
    tree.vcs(&["init", "-q"]);
    let a = tree.commit("A");
    succeeded(&tree.pin(&[]));
    // B: `box_drop`'s body changes in a way its contract still covers.
    tree.replace(BOX_DROP_A, BOX_DROP_B);
    let b = tree.commit("B");
    let check = tree.pin(&["--check"]);
    assert_eq!(check.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&check.stdout);
    assert!(
        stdout.contains("x box_drop: not in force (source-changed)"),
        "{stdout}"
    );
    assert!(
        stdout.contains("9 in force through another revision"),
        "{stdout}"
    );
    succeeded(&tree.pin(&[]));
    succeeded(&tree.pin(&["--check"]));
    // `box_drop` has two identities; every other entry has one identity
    // reviewed at both revisions.
    let entry = tree.entry("box_drop");
    let identities = entry["identities"].as_array().unwrap();
    assert_eq!(identities.len(), 2);
    let at = |identity: &Value| identity["reviewed_at"][0]["revision"].clone();
    let revisions = identities.iter().map(at).collect::<Vec<_>>();
    assert!(
        revisions.contains(&Value::from(a.clone())) && revisions.contains(&Value::from(b.clone()))
    );
    let keep = tree.entry("box_keep");
    assert_eq!(keep["identities"].as_array().unwrap().len(), 1);
    assert_eq!(
        keep["identities"][0]["reviewed_at"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    // In force at B, through the identity reviewed at B.
    let section = tree.audit("at-b/r.json");
    assert!(mismatches(&section).is_empty());
    let in_force = section["pinned_in_force"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["function"] == "box_drop")
        .unwrap()
        .clone();
    assert_eq!(in_force["identities"], 2);
    assert_eq!(in_force["matched_reviewed_at"][0]["revision"], b.as_str());
    assert_eq!(tree.status("at-b/r.json", "user_drop").0, "Safe");
    // Adding B did not disturb A.
    tree.vcs(&["checkout", "-q", &a]);
    let section = tree.audit("at-a/r.json");
    assert!(mismatches(&section).is_empty());
    assert_eq!(tree.status("at-a/r.json", "user_drop").0, "Safe");
    succeeded(&tree.pin(&["--check"]));
    // C: a body reviewed at neither revision.
    tree.vcs(&["checkout", "-q", "-b", "c"]);
    tree.replace(BOX_DROP_A, BOX_DROP_C);
    let c = tree.commit("C");
    let section = tree.audit("at-c/r.json");
    assert_eq!(
        mismatches(&section),
        [("box_drop".to_owned(), "source-changed".to_owned())]
    );
    // Card W45-USER-CONTRACT-PLUGINS made a caller resting only on the held
    // call Pending (it was Unknown). Card
    // W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL corrects this one back to
    // Unknown: C drops `box_drop`'s release, so a re-pin as written would
    // leave the declaration refuted by its own body and the call withdrawn;
    // the reason that would stand is recorded beside the held call.
    assert_eq!(
        tree.status("at-c/r.json", "user_drop"),
        (
            "Unknown".to_owned(),
            vec!["contract-pin-mismatch".to_owned(), "unsupported".to_owned()]
        )
    );
    let detail = section["pin_mismatches"][0]["detail"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(detail.contains(&a) && detail.contains(&b), "{detail}");
    // `--check --revision` reads a revision's blobs without a checkout.
    succeeded(&tree.pin(&["--check", "--revision", &a]));
    let at_c = tree.pin(&["--check", "--revision", &c]);
    assert_eq!(at_c.status.code(), Some(1));
    // Dropping A: back at A, `box_drop` is out of force; every other entry
    // stays in force through its identity reviewed at B.
    succeeded(&tree.quirin(&["pin", "--modules", "contracts", "--drop-revision", &a[..10]]));
    assert_eq!(
        tree.entry("box_drop")["identities"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    tree.vcs(&["checkout", "-q", &a]);
    let section = tree.audit("dropped-a/r.json");
    assert_eq!(
        mismatches(&section),
        [("box_drop".to_owned(), "source-changed".to_owned())]
    );
    // A contract edit voids every identity, whatever revision is audited.
    let path = tree.0.join("contracts/queue.qv");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replacen(
            "resource_released(item, nullable)",
            "resource_released(item, nonnull)",
            1,
        ),
    )
    .unwrap();
    for revision in [&a, &b] {
        tree.vcs(&["checkout", "-q", revision]);
        let section = tree.audit(&format!("edited-{}/r.json", &revision[..7]));
        assert!(
            mismatches(&section).contains(&("item_free".to_owned(), "contract-changed".to_owned())),
            "{revision}"
        );
    }
    let repin = tree.pin(&["--function", "item_free"]);
    succeeded(&repin);
    assert!(String::from_utf8_lossy(&repin.stdout).contains("! item_free: contract text changed"));
    let item_free = tree.entry("item_free");
    assert_eq!(item_free["identities"].as_array().unwrap().len(), 1);
    assert_eq!(
        item_free["identities"][0]["reviewed_at"],
        serde_json::json!([{"kind": "git", "revision": b}])
    );
    // `--regenerate` rebuilds each lock from this revision alone.
    succeeded(&tree.pin(&["--regenerate"]));
    let lock = tree.read("contracts/contracts.lock");
    assert!(lock["pinned"].as_array().unwrap().iter().all(|e| {
        e["identities"]
            == serde_json::json!([{
                "reviewed_at": [{"kind": "git", "revision": b}],
                "sites": e["identities"][0]["sites"],
            }])
    }));
    // A revision the lock never recorded names nothing.
    let refused = tree.quirin(&["pin", "--modules", "contracts", "--drop-revision", &c[..10]]);
    assert_eq!(refused.status.code(), Some(2));
}

#[test]
fn a_version_one_lock_is_read_as_one_identity_per_entry() {
    // One explicit stamp for both runs: a default (clock) stamp makes the
    // byte comparison below depend on both runs falling in the same second.
    const STAMP: &str = "2026-10-25T18:10:00Z";
    let tree = Tree::new();
    succeeded(&tree.pin(&["--pinned-at", STAMP]));
    let path = tree.0.join("contracts/contracts.lock");
    let lock: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let v1 = serde_json::json!({
        "lock_version": 1,
        "generated_by": lock["generated_by"],
        "note": lock["note"],
        "owner": lock["owner"],
        "source": {"kind": "directory", "normalisation": lock["normalisation"]},
        "pinned": lock["pinned"].as_array().unwrap().iter().map(|e| serde_json::json!({
            "function": e["function"], "module": e["module"],
            "contract_sha256": e["contract_sha256"], "sites": e["identities"][0]["sites"],
        })).collect::<Vec<_>>(),
        "flexible": lock["flexible"],
    });
    fs::write(&path, serde_json::to_string_pretty(&v1).unwrap()).unwrap();
    succeeded(&tree.pin(&["--check"]));
    // Re-pinning writes the current version, identical to the lock it was made from.
    succeeded(&tree.pin(&["--pinned-at", STAMP]));
    assert_eq!(tree.read("contracts/contracts.lock"), lock);
}

#[test]
fn a_malformed_version_two_lock_refuses() {
    let tree = Tree::new();
    succeeded(&tree.pin(&[]));
    let path = tree.0.join("contracts/contracts.lock");
    let lock: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    // One source in two identities of an entry.
    let mut twice = lock.clone();
    let identity = twice["pinned"][0]["identities"][0].clone();
    let mut other = identity.clone();
    other["sites"] = serde_json::json!([]);
    twice["pinned"][0]["identities"] = serde_json::json!([identity, other]);
    // An identity with no source; an unknown version.
    let mut empty = lock.clone();
    empty["pinned"][0]["identities"][0]["reviewed_at"] = serde_json::json!([]);
    // Card CONTRACT-PIN-TIP-MODE made version 3 the tip-entry format and card
    // CONTRACT-SUPERSESSION version 4 the stamped one, so the unknown version
    // is 5.
    let mut version = lock.clone();
    version["lock_version"] = serde_json::json!(5);
    for (bad, expected) in [
        (twice, "malformed entry"),
        (empty, "malformed entry"),
        (version, "unsupported lock_version"),
    ] {
        fs::write(&path, serde_json::to_string(&bad).unwrap()).unwrap();
        let output = tree.pin(&["--check"]);
        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected), "{stderr}");
    }
}
