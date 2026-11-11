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

//! Card PIN-REGENERATE-STAMP-MONOTONICITY (contract spec §C06.6): a re-pin
//! carries a later stamp than the entry it replaces, an explicit stamp that
//! is not later refuses, and a default (clock) stamp behind it moves one
//! second past it. `pin --regenerate` rebuilds each lock from one source, so
//! the entry it replaces is the one of the lock on disk before the run; the
//! same holds for an entry whose revision mode changes (the entry of the
//! other kind is the replaced one). A refusal writes no lock.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

static NEXT: AtomicUsize = AtomicUsize::new(0);

const FLEXIBLE_MARKER: &str = "    requires resource_revision(flexible);\n";
const EARLY: &str = "2026-10-24T00:00:00Z";
const PINNED: &str = "2026-10-25T18:10:00Z";
const LATER: &str = "2026-10-26T00:00:00Z";
const AHEAD: &str = "2999-01-01T00:00:00Z";

fn clang_available() -> bool {
    if qcore_c_gateway::clang_command().is_ok() {
        return true;
    }
    assert!(
        std::env::var_os("QCORE_CLANG").is_none(),
        "explicit QCORE_CLANG is invalid"
    );
    eprintln!("skipping Clang-dependent pin-regeneration stamp test");
    false
}

fn fixture(name: &str) -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/revision_mode")
            .join(name),
    )
    .unwrap()
    .replace("\r\n", "\n")
}

/// The fixture's contracts with `item_new` flexible (when `flexible`) and
/// every other declaration pinned.
fn contracts(flexible: bool) -> String {
    let text = fixture("queue.qv");
    assert!(text.matches(FLEXIBLE_MARKER).count() > 1);
    let kept = if flexible { FLEXIBLE_MARKER } else { "" };
    text.replacen(FLEXIBLE_MARKER, "@@KEPT@@", 1)
        .replace(FLEXIBLE_MARKER, "")
        .replace("@@KEPT@@", kept)
}

/// A fresh tree: `src/queue.c` (the fixture) and `contracts/queue.qv`.
struct Tree(PathBuf);

impl Tree {
    fn new(flexible: bool) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-pin-regenerate-stamps-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("contracts")).unwrap();
        fs::write(root.join("src/queue.c"), fixture("queue.c")).unwrap();
        fs::write(root.join("contracts/queue.qv"), contracts(flexible)).unwrap();
        Self(root)
    }

    fn pin(&self, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(&self.0)
            .args(["pin", "--source-root", "src", "--modules", "contracts"])
            .args(extra)
            .output()
            .unwrap()
    }

    fn lock_bytes(&self) -> Vec<u8> {
        fs::read(self.0.join("contracts/contracts.lock")).unwrap()
    }

    fn lock(&self) -> Value {
        serde_json::from_slice(&self.lock_bytes()).unwrap()
    }

    /// Every entry's (kind, function, stamp).
    fn stamps(&self) -> Vec<(String, String, String)> {
        let lock = self.lock();
        let mut stamps = Vec::new();
        for kind in ["pinned", "flexible", "tip"] {
            for entry in lock[kind].as_array().into_iter().flatten() {
                stamps.push((
                    kind.to_owned(),
                    entry["function"].as_str().unwrap().to_owned(),
                    entry["pinned_at"].as_str().unwrap().to_owned(),
                ));
            }
        }
        stamps
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

fn refused_as_not_later(output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(2),
        "stdout: {}\nstderr: {stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        stderr.contains("a re-pin must carry a later stamp than the entry it replaces"),
        "{stderr}"
    );
}

fn all_stamped(tree: &Tree, stamp: &str) {
    let stamps = tree.stamps();
    assert!(
        stamps.iter().any(|(kind, _, _)| kind == "pinned")
            && stamps.iter().any(|(kind, _, _)| kind == "flexible"),
        "{stamps:?}"
    );
    assert!(stamps.iter().all(|(_, _, s)| s == stamp), "{stamps:?}");
}

#[test]
fn regeneration_refuses_an_explicit_stamp_not_later_than_the_one_it_replaces() {
    if !clang_available() {
        return;
    }
    let tree = Tree::new(true);
    succeeded(&tree.pin(&["--pinned-at", PINNED]));
    all_stamped(&tree, PINNED);
    let before = tree.lock_bytes();
    // Earlier, then equal: refused, and the lock is not written.
    for stamp in [EARLY, PINNED] {
        refused_as_not_later(&tree.pin(&["--regenerate", "--pinned-at", stamp]));
        assert_eq!(tree.lock_bytes(), before, "{stamp}");
    }
    // Later: every regenerated entry carries it.
    succeeded(&tree.pin(&["--regenerate", "--pinned-at", LATER]));
    all_stamped(&tree, LATER);
}

#[test]
fn regeneration_moves_a_default_stamp_past_a_lock_ahead_of_the_clock() {
    if !clang_available() {
        return;
    }
    let tree = Tree::new(true);
    succeeded(&tree.pin(&["--pinned-at", AHEAD]));
    // The clock is behind every stamp of the lock: each regenerated entry
    // moves one second past the stamp it replaces.
    succeeded(&tree.pin(&["--regenerate"]));
    all_stamped(&tree, "2999-01-01T00:00:01Z");
}

#[test]
fn a_mode_change_replaces_the_entry_of_the_other_kind() {
    if !clang_available() {
        return;
    }
    // `item_new` flexible at PINNED, then made pinned: its new pinned entry
    // replaces the flexible one, under an update and under a regeneration.
    for regenerate in [false, true] {
        let tree = Tree::new(true);
        succeeded(&tree.pin(&["--pinned-at", PINNED]));
        fs::write(tree.0.join("contracts/queue.qv"), contracts(false)).unwrap();
        let before = tree.lock_bytes();
        let mut args = vec!["--pinned-at", EARLY];
        if regenerate {
            args.push("--regenerate");
        }
        refused_as_not_later(&tree.pin(&args));
        assert_eq!(tree.lock_bytes(), before, "regenerate {regenerate}");
        // A default stamp behind the replaced one moves past it.
        let tree = Tree::new(true);
        succeeded(&tree.pin(&["--pinned-at", AHEAD]));
        fs::write(tree.0.join("contracts/queue.qv"), contracts(false)).unwrap();
        let mut args = Vec::new();
        if regenerate {
            args.push("--regenerate");
        }
        succeeded(&tree.pin(&args));
        let stamps = tree.stamps();
        assert!(
            stamps.contains(&(
                "pinned".to_owned(),
                "item_new".to_owned(),
                "2999-01-01T00:00:01Z".to_owned()
            )),
            "regenerate {regenerate}: {stamps:?}"
        );
        assert!(
            stamps.iter().all(|(_, _, s)| s.as_str() >= AHEAD),
            "regenerate {regenerate}: {stamps:?}"
        );
    }
}
