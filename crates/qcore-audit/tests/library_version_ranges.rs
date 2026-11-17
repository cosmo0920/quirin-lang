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

//! Card OPENSSL-PROVIDER-PLUGIN, end to end: a user crate's declaration
//! stating a library version range (`resource_library_version`), its version
//! source (`library-versions.json`), `quirin-audit pin` recording the probed
//! version in the crate's lock, and `quirin-audit contracts` deciding the
//! declaration's standing from the audited headers. The user decision's
//! fixtures: in range -> in force; below `since` -> held; past removal ->
//! held; version unreadable -> held; header version != lock version ->
//! mismatch (Pending); and two include configurations disagreeing ->
//! ambiguous (held).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn clang_available() -> bool {
    if qcore_c_gateway::clang_command().is_ok() {
        return true;
    }
    assert!(
        std::env::var_os("QCORE_CLANG").is_none(),
        "explicit QCORE_CLANG is invalid"
    );
    eprintln!("skipping Clang-dependent library version test");
    false
}

const RANGE: &str = "requires resource_library_version(fakelib, since \"2.0\");";

struct Tree(PathBuf);

impl Tree {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-library-version-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/library_version");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::copy(
            fixture.join("src/reset_user.c"),
            root.join("src/reset_user.c"),
        )
        .unwrap();
        fs::create_dir_all(root.join("include/fakelib")).unwrap();
        fs::copy(
            fixture.join("include/fakelib/fakelibv.h"),
            root.join("include/fakelib/fakelibv.h"),
        )
        .unwrap();
        let krate = root.join("user/quirin-provider-fakelib");
        fs::create_dir_all(krate.join("src")).unwrap();
        fs::create_dir_all(krate.join("contracts/fakelib")).unwrap();
        fs::write(
            krate.join("Cargo.toml"),
            "[package]\nname = \"quirin-provider-fakelib\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::write(
            krate.join("src/lib.rs"),
            "pub const NAMESPACE: &str = \"fakelib\";\npub const PUBLISHED: &[&str] = &[];\n",
        )
        .unwrap();
        fs::write(
            krate.join("library-versions.json"),
            serde_json::to_vec_pretty(&json!({
                "schema_version": 1,
                "libraries": {"fakelib": {"header": "fakelib/fakelibv.h",
                    "major": "FAKELIB_VERSION_MAJOR", "minor": "FAKELIB_VERSION_MINOR",
                    "patch": "FAKELIB_VERSION_PATCH"}},
            }))
            .unwrap(),
        )
        .unwrap();
        let tree = Self(root);
        tree.contract(RANGE);
        fs::write(
            tree.0.join("user/user-load-path.json"),
            serde_json::to_vec_pretty(&json!({
                "schema_version": 1,
                "extends": repository().join("providers/contract-load-path.json"),
                "crates": [{
                    "name": "quirin-provider-fakelib", "namespace": "fakelib",
                    "version": "0.1.0", "contracts": "quirin-provider-fakelib/contracts",
                    "dependencies": [], "modules": ["fakelib/reset.qv"], "published": {},
                }],
                "overrides": [],
                "checkers": [],
            }))
            .unwrap(),
        )
        .unwrap();
        tree.database(&["include"]);
        succeeded(&tree.quirin(&[
            "plugin",
            "views",
            "--load-path",
            "user/user-load-path.json",
            "--write",
        ]));
        tree
    }

    fn contract(&self, range: &str) {
        fs::write(
            self.0
                .join("user/quirin-provider-fakelib/contracts/fakelib/reset.qv"),
            format!(
                "module contract;\n\nextern fn fakelib_reset() -> unit\ncontract {{\n    \
                 requires resource_contract_v1;\n    requires resource_revision(flexible);\n    \
                 {range}\n    ensures normal resource_return_none();\n}};\n"
            ),
        )
        .unwrap();
    }

    /// A compile database with one command per include directory.
    fn database(&self, includes: &[&str]) {
        let entries = includes
            .iter()
            .enumerate()
            .map(|(index, include)| {
                json!({"directory": self.0, "file": format!("src/unit{index}.c"),
                       "arguments": ["cc", format!("-I{include}"), "-c",
                                     format!("src/unit{index}.c")]})
            })
            .collect::<Vec<_>>();
        fs::write(
            self.0.join("compile_commands.json"),
            serde_json::to_vec(&entries).unwrap(),
        )
        .unwrap();
    }

    fn header(&self, directory: &str, major: u32, minor: u32, patch: u32) {
        fs::create_dir_all(self.0.join(directory).join("fakelib")).unwrap();
        fs::write(
            self.0.join(directory).join("fakelib/fakelibv.h"),
            format!(
                "#define FAKELIB_VERSION_MAJOR {major}\n#define FAKELIB_VERSION_MINOR \
                 {minor}\n#define FAKELIB_VERSION_PATCH {patch}\n"
            ),
        )
        .unwrap();
    }

    fn quirin(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(&self.0)
            .env(
                "QCORE_LIBRARY_VERSION_COMPILE_DATABASE",
                self.0.join("compile_commands.json"),
            )
            .args(args)
            .output()
            .unwrap()
    }

    fn pin(&self) {
        succeeded(&self.quirin(&[
            "pin",
            "--load-path",
            "user/user-load-path.json",
            "--source-root",
            "src",
        ]));
    }

    /// Audit; the use_reset row's status and the declaration's record.
    fn audit(&self, report: &str) -> (String, Value) {
        succeeded(&self.quirin(&[
            "contracts",
            "--source-root",
            "src",
            "--load-path",
            "user/user-load-path.json",
            "--provider",
            "quirin-provider-fakelib",
            "--report",
            report,
        ]));
        let read = |path: &str| -> Value {
            serde_json::from_slice(&fs::read(self.0.join(path)).unwrap()).unwrap()
        };
        let rows = read(report);
        let unit = rows["units"]
            .as_array()
            .unwrap()
            .iter()
            .find(|unit| unit["file"] == "reset_user.c")
            .unwrap()
            .clone();
        let status = unit["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["function"] == "use_reset")
            .unwrap()["status"]
            .as_str()
            .unwrap()
            .to_owned();
        let sidecar = read(&report.replace(".json", ".contracts.json"));
        let record = sidecar["contract_revision"]["library_versions"]["declarations"][0].clone();
        (status, record)
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
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_range_is_in_force_only_at_the_pinned_version_inside_it() {
    if !clang_available() {
        return;
    }
    let tree = Tree::new();
    // No lock yet: the version is not pinned, a mismatch a re-pin resolves.
    let (status, record) = tree.audit("unpinned.json");
    assert_eq!(status, "Pending", "{record}");
    assert_eq!(record["reason"], "library-version-changed");
    // `pin` records the probed version; in range and pinned: in force.
    tree.pin();
    let lock: Value = serde_json::from_slice(
        &fs::read(tree.0.join("user/quirin-provider-fakelib/contracts.lock")).unwrap(),
    )
    .unwrap();
    assert_eq!(lock["library_versions"]["fakelib"]["version"], "2.3.1");
    let (status, record) = tree.audit("in-range.json");
    assert_eq!(status, "Safe", "{record}");
    assert_eq!(record["in_force"], true);
    assert_eq!(record["audited_version"], "2.3.1");
    assert_eq!(record["pinned_version"], "2.3.1");
    // The headers move to another version inside the range: a mismatch with
    // the lock, Pending until a review re-pins.
    tree.header("include", 2, 4, 0);
    let (status, record) = tree.audit("changed.json");
    assert_eq!(status, "Pending", "{record}");
    assert_eq!(record["reason"], "library-version-changed");
    tree.header("include", 2, 3, 1);
    // Below `since`: held, Unknown.
    tree.contract("requires resource_library_version(fakelib, since \"2.4\");");
    let (status, record) = tree.audit("below.json");
    assert_eq!(status, "Unknown", "{record}");
    assert_eq!(record["reason"], "library-version-out-of-range");
    // Past the removal: held, Unknown.
    tree.contract("requires resource_library_version(fakelib, since \"2.0\", removed \"2.3.1\");");
    let (status, record) = tree.audit("removed.json");
    assert_eq!(status, "Unknown", "{record}");
    assert_eq!(record["reason"], "library-version-out-of-range");
    tree.contract(RANGE);
    // The headers cannot be read: held, Unknown.
    tree.database(&["missing"]);
    let (status, record) = tree.audit("unreadable.json");
    assert_eq!(status, "Unknown", "{record}");
    assert_eq!(record["reason"], "library-version-unreadable");
    // Two include configurations report different versions: held, Unknown.
    tree.header("other", 3, 0, 0);
    tree.database(&["include", "other"]);
    let (status, record) = tree.audit("ambiguous.json");
    assert_eq!(status, "Unknown", "{record}");
    assert_eq!(record["reason"], "library-version-ambiguous");
    // Back to the pinned configuration: in force again.
    tree.database(&["include"]);
    let (status, _) = tree.audit("restored.json");
    assert_eq!(status, "Safe");
}

#[test]
fn a_run_without_a_range_writes_no_record() {
    if !clang_available() {
        return;
    }
    let tree = Tree::new();
    tree.contract("");
    tree.pin();
    let (status, record) = tree.audit("plain.json");
    assert_eq!(status, "Safe");
    assert!(record.is_null(), "{record}");
    let lock =
        fs::read_to_string(tree.0.join("user/quirin-provider-fakelib/contracts.lock")).unwrap();
    assert!(!lock.contains("library_versions"), "{lock}");
}
