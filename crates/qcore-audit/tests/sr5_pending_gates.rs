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

//! Card W45-SR5-SOUNDNESS (`PENDING-BYPASSES-UNIT-SAFE-GATES`), on the audit
//! route: a unit whose only unresolved diagnostics are held calls is `Pending`
//! only when it passes the `Safe` arm's scope gates too. A selected definition
//! the analyzer does not read (a K&R body) beside a held call makes the unit
//! `Unknown`; the function row keeps its own held reason.
//!
//! The fixture is W45-CONTRACT-REVISION-MODE's held-out
//! `tests/fixtures/revision_mode/` (derived from no production source): a
//! body change to `box_drop` puts it out of force, so `user_drop` holds a call
//! for a pin mismatch. Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL: the
//! change is a second `item_touch` that keeps the release (the identities
//! test's revision B), so a re-pin as written leaves `user_drop` `Safe` and its
//! row `Pending`. The `dropped-release` mutation this test used before would
//! leave the re-pinned declaration refuted by its body, and `user_drop` is
//! `Unknown` there since that card.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

static NEXT: AtomicUsize = AtomicUsize::new(0);

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
    eprintln!("skipping Clang-dependent W45-SR5-SOUNDNESS audit test");
    false
}

struct Tree(PathBuf);

impl Tree {
    /// The fixture with every contract pinned, and `extra` appended to the
    /// source before the lock is written.
    fn new(extra: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-sr5-pending-gates-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("contracts")).unwrap();
        fs::write(
            root.join("src/queue.c"),
            format!("{}{extra}", fixture("queue.c")),
        )
        .unwrap();
        fs::write(
            root.join("contracts/queue.qv"),
            fixture("queue.qv").replace("    requires resource_revision(flexible);\n", ""),
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

    /// Pin, then change `box_drop`'s body (keeping its release), then audit.
    fn held_report(&self, report: &str) -> Value {
        succeeded(&self.quirin(&["pin", "--source-root", "src", "--modules", "contracts"]));
        let (old, new) = (
            "    item_touch(item);\n    item_free(item);",
            "    item_touch(item);\n    item_touch(item);\n    item_free(item);",
        );
        let path = self.0.join("src/queue.c");
        let mut text = fs::read_to_string(&path).unwrap();
        assert_eq!(text.matches(old).count(), 1, "{old}");
        text = text.replacen(old, new, 1);
        fs::write(path, text).unwrap();
        succeeded(&self.quirin(&[
            "contracts",
            "--source-root",
            "src",
            "--contract-manifest",
            "manifest.json",
            "--report",
            report,
        ]));
        serde_json::from_slice(&fs::read(self.0.join(report)).unwrap()).unwrap()
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

fn function_status<'a>(report: &'a Value, name: &str) -> &'a str {
    report["units"][0]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["function"] == name)
        .unwrap()["status"]
        .as_str()
        .unwrap()
}

#[test]
fn a_held_call_beside_an_unread_definition_is_an_unknown_unit() {
    if !clang_available() {
        return;
    }
    // The control: the held call alone is a Pending unit.
    let control = Tree::new("");
    let report = control.held_report("control/r.json");
    assert_eq!(function_status(&report, "user_drop"), "Pending");
    assert_eq!(
        report["units"][0]["status"], "Pending",
        "{}",
        report["units"][0]
    );
    // Beside a K&R definition the analyzer does not read: Unknown.
    let gated = Tree::new("\nint kr_width(width) int width; { return width; }\n");
    let report = gated.held_report("gated/r.json");
    assert_eq!(function_status(&report, "user_drop"), "Pending");
    assert_eq!(
        report["units"][0]["status"], "Unknown",
        "{}",
        report["units"][0]
    );
    assert!(
        report["units"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("carry no analysed row"),
        "{}",
        report["units"][0]["reason"]
    );
}
