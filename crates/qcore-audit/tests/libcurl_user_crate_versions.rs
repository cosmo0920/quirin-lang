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

//! Card LIBCURL-PROVIDER-PLUGIN, end to end: the real user crate
//! `providers/quirin-provider-libcurl` (its committed contract modules and
//! `library-versions.json`) on a scratch user load path, `quirin-audit pin`
//! recording the probed `curl/curlver.h` version, and `quirin-audit contracts`
//! deciding each declaration's standing (rule 53) as the audited headers move:
//! the pinned version in range -> in force; a changed version inside the range
//! -> `Pending` (`library-version-changed`); below a declaration's documented
//! `since` -> `Unknown` (`library-version-out-of-range`); the header not found
//! -> `Unknown` (`library-version-unreadable`).
//!
//! The scratch load path binds no checker, so the range decision is isolated
//! from the plugin firewall: a declaration in force here is not a published
//! `Safe` claim. On the generated user load path every declaration is behind
//! the crate's checker, whose `Safe` is `Pending` (`plugin-claim-uncertified`;
//! `user_contract_plugins.rs`), and `providers/contracts.py check` refuses a
//! declaration no checker binds.

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
    eprintln!("skipping Clang-dependent libcurl user crate test");
    false
}

const MODULES: [&str; 4] = [
    "libcurl/escape.qv",
    "libcurl/global.qv",
    "libcurl/slist.qv",
    "libcurl/url.qv",
];

struct Tree(PathBuf);

impl Tree {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-libcurl-user-crate-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/libcurl_user_crate");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::copy(
            fixture.join("src/curl_user.c"),
            root.join("src/curl_user.c"),
        )
        .unwrap();
        let source = repository().join("providers/quirin-provider-libcurl");
        let krate = root.join("user/quirin-provider-libcurl");
        fs::create_dir_all(krate.join("src")).unwrap();
        fs::create_dir_all(krate.join("contracts/libcurl")).unwrap();
        for module in MODULES {
            fs::copy(
                source.join("contracts").join(module),
                krate.join("contracts").join(module),
            )
            .unwrap();
        }
        fs::copy(
            source.join("library-versions.json"),
            krate.join("library-versions.json"),
        )
        .unwrap();
        fs::write(
            krate.join("Cargo.toml"),
            "[package]\nname = \"quirin-provider-libcurl\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::write(
            krate.join("src/lib.rs"),
            "pub const NAMESPACE: &str = \"libcurl\";\npub const PUBLISHED: &[&str] = &[];\n",
        )
        .unwrap();
        fs::write(
            root.join("user/user-load-path.json"),
            serde_json::to_vec_pretty(&json!({
                "schema_version": 1,
                "extends": repository().join("providers/contract-load-path.json"),
                "crates": [{
                    "name": "quirin-provider-libcurl", "namespace": "libcurl",
                    "version": "0.1.0", "contracts": "quirin-provider-libcurl/contracts",
                    "dependencies": [], "modules": MODULES, "published": {},
                }],
                "overrides": [],
                "checkers": [],
            }))
            .unwrap(),
        )
        .unwrap();
        let tree = Self(root);
        tree.header(8, 18, 0);
        fs::write(
            tree.0.join("compile_commands.json"),
            serde_json::to_vec(&json!([{
                "directory": tree.0, "file": "src/curl_user.c",
                "arguments": ["cc", "-isystem", "include", "-c", "src/curl_user.c"],
            }]))
            .unwrap(),
        )
        .unwrap();
        succeeded(&tree.quirin(&[
            "plugin",
            "views",
            "--load-path",
            "user/user-load-path.json",
            "--write",
        ]));
        tree
    }

    /// `include/curl/curlver.h` reporting this version, as libcurl spells it.
    fn header(&self, major: u32, minor: u32, patch: u32) {
        fs::create_dir_all(self.0.join("include/curl")).unwrap();
        fs::write(
            self.0.join("include/curl/curlver.h"),
            format!(
                "#ifndef CURLINC_CURLVER_H\n#define CURLINC_CURLVER_H\n#define LIBCURL_VERSION \
                 \"{major}.{minor}.{patch}\"\n#define LIBCURL_VERSION_MAJOR {major}\n#define \
                 LIBCURL_VERSION_MINOR {minor}\n#define LIBCURL_VERSION_PATCH {patch}\n#endif\n"
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
            "--crate",
            "quirin-provider-libcurl",
        ]));
    }

    /// Audit; each fixture function's status and each declaration's record.
    fn audit(&self, report: &str) -> (Value, Value) {
        succeeded(&self.quirin(&[
            "contracts",
            "--source-root",
            "src",
            "--load-path",
            "user/user-load-path.json",
            "--provider",
            "quirin-provider-libc",
            "--provider",
            "quirin-provider-libcurl",
            "--report",
            report,
        ]));
        let read = |path: &str| -> Value {
            serde_json::from_slice(&fs::read(self.0.join(path)).unwrap()).unwrap()
        };
        let rows = read(report);
        let mut statuses = serde_json::Map::new();
        for unit in rows["units"].as_array().unwrap() {
            for row in unit["functions"].as_array().unwrap() {
                statuses.insert(
                    row["function"].as_str().unwrap().to_owned(),
                    row["status"].clone(),
                );
            }
        }
        let sidecar = read(&report.replace(".json", ".contracts.json"));
        let mut records = serde_json::Map::new();
        for record in sidecar["contract_revision"]["library_versions"]["declarations"]
            .as_array()
            .unwrap()
        {
            records.insert(
                record["function"].as_str().unwrap().to_owned(),
                record.clone(),
            );
        }
        (Value::Object(statuses), Value::Object(records))
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
fn the_libcurl_ranges_follow_the_audited_curlver_header() {
    if !clang_available() {
        return;
    }
    let tree = Tree::new();
    // `pin` records the probed version: 8.18.0, the audited build's.
    tree.pin();
    let lock: Value = serde_json::from_slice(
        &fs::read(tree.0.join("user/quirin-provider-libcurl/contracts.lock")).unwrap(),
    )
    .unwrap();
    assert_eq!(lock["library_versions"]["curl"]["version"], "8.18.0");
    assert_eq!(lock["flexible"].as_array().unwrap().len(), 6);
    // In range and pinned: every applied declaration is in force.
    let (statuses, records) = tree.audit("in-range.json");
    for function in ["curl_global_init", "curl_url", "curl_url_cleanup"] {
        assert_eq!(records[function]["in_force"], true, "{records}");
        assert_eq!(records[function]["audited_version"], "8.18.0");
        assert_eq!(records[function]["pinned_version"], "8.18.0");
    }
    assert_eq!(
        records["curl_url"]["range"],
        "resource_library_version(curl, since \"7.62.0\")"
    );
    assert_eq!(
        records["curl_global_init"]["range"],
        "resource_library_version(curl, since \"7.8.0\")"
    );
    assert_ne!(statuses["init_once"], "Unknown", "{statuses}");
    assert_ne!(statuses["url_cycle"], "Unknown", "{statuses}");
    // The headers move to 8.19.0, inside every range: a mismatch with the
    // lock, Pending until a review re-pins.
    tree.header(8, 19, 0);
    let (statuses, records) = tree.audit("changed.json");
    assert_eq!(statuses["init_once"], "Pending", "{records}");
    assert_eq!(statuses["url_cycle"], "Pending", "{records}");
    assert_eq!(records["curl_url"]["reason"], "library-version-changed");
    // 7.61.0: below curl_url's documented `since` 7.62.0, held Unknown;
    // curl_global_init (since 7.8) is in range but not the pinned version.
    tree.header(7, 61, 0);
    let (statuses, records) = tree.audit("below.json");
    assert_eq!(statuses["url_cycle"], "Unknown", "{records}");
    assert_eq!(
        records["curl_url"]["reason"],
        "library-version-out-of-range"
    );
    assert_eq!(
        records["curl_url_cleanup"]["reason"],
        "library-version-out-of-range"
    );
    assert_eq!(statuses["init_once"], "Pending", "{records}");
    assert_eq!(
        records["curl_global_init"]["reason"],
        "library-version-changed"
    );
    // 7.7.0: below curl_global_init's 7.8 as well.
    tree.header(7, 7, 0);
    let (statuses, records) = tree.audit("older.json");
    assert_eq!(statuses["init_once"], "Unknown", "{records}");
    assert_eq!(
        records["curl_global_init"]["reason"],
        "library-version-out-of-range"
    );
    // No curl/curlver.h at all: unreadable, every call held Unknown.
    fs::remove_file(tree.0.join("include/curl/curlver.h")).unwrap();
    let (statuses, records) = tree.audit("unreadable.json");
    assert_eq!(statuses["init_once"], "Unknown", "{records}");
    assert_eq!(statuses["url_cycle"], "Unknown", "{records}");
    assert_eq!(
        records["curl_global_init"]["reason"],
        "library-version-unreadable"
    );
    // Back to the pinned headers: in force again.
    tree.header(8, 18, 0);
    let (_, records) = tree.audit("restored.json");
    assert_eq!(records["curl_url"]["in_force"], true, "{records}");
}

/// Rule 53: the crate is listed on the generated user load path, never on the
/// shipped one.
#[test]
fn the_libcurl_crate_rides_the_user_load_path_only() {
    let read = |name: &str| -> String {
        fs::read_to_string(repository().join("providers").join(name)).unwrap()
    };
    let shipped = read("contract-load-path.json");
    assert!(!shipped.contains("quirin-provider-libcurl"));
    let user: Value = serde_json::from_str(&read("user-crates-load-path.json")).unwrap();
    assert!(
        user["crates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| record["name"] == "quirin-provider-libcurl")
    );
}
