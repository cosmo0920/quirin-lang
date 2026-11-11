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

//! Card AUDIT-CANONICAL-PATHS (user decision 7): `quirin-audit current`
//! selects units by located path, so a compile database spelled through a
//! symlink to a moved checkout keeps working, while a record that does not
//! resolve, resolves outside the checkout, or spells one source two ways is
//! refused and never admitted.
//!
//! The regression it fixes (`PINNED-INPUTS-PATH-AFTER-MIGRATION`): with a
//! cross-unit link map, a database spelled through the old symlink failed
//! with `cross-unit home lies outside checkout` before any analysis.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);
static CLANG: OnceLock<Result<PathBuf, String>> = OnceLock::new();

/// The configured Clang, or `None` (and a skip) when none is available.
fn clang() -> Option<PathBuf> {
    match CLANG.get_or_init(|| {
        qcore_c_gateway::clang_command().map(|command| PathBuf::from(command.get_program()))
    }) {
        Ok(path) => Some(path.clone()),
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent canonical-path test: {reason}");
            None
        }
    }
}

const HEAP: &str = "module contract;\n\
extern fn alloc() -> ptr<mut, u8>\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_return_owned(nonnull);\n\
};\n\
extern fn dispose(p: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_released(p, nonnull);\n\
    ensures normal resource_return_none();\n\
};\n";

const UNITS: [(&str, &str); 2] = [
    (
        "src/home.c",
        "unsigned char *alloc(void);\nvoid dispose(unsigned char *p);\n\
         void release(unsigned char *p) { dispose(p); }\n",
    ),
    (
        "src/caller.c",
        "unsigned char *alloc(void);\nvoid release(unsigned char *p);\n\
         void subject(int c) { unsigned char *p = alloc(); if (c) release(p); else release(p); }\n",
    ),
];

fn git(directory: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A committed checkout `checkout/`, the old spelling `old/` (a symlink to
/// it, as `/home/cosmo/Documents/GitHub/qcore-c` is to the moved checkout),
/// a top-level source profile, one contract module and a link map naming
/// `release`'s home.
struct Fixture {
    root: PathBuf,
    driver: PathBuf,
}

impl Fixture {
    fn new(driver: &Path) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-canonical-paths-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let checkout = root.join("checkout");
        fs::create_dir_all(checkout.join("src")).unwrap();
        fs::create_dir_all(checkout.join("build")).unwrap();
        for (path, text) in UNITS {
            fs::write(checkout.join(path), text).unwrap();
        }
        git(&checkout, &["init", "--quiet"]);
        git(&checkout, &["add", "src"]);
        git(
            &checkout,
            &[
                "-c",
                "user.name=Audit Fixture",
                "-c",
                "user.email=audit@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Canonical-path fixture",
            ],
        );
        let commit = String::from_utf8(
            Command::new("git")
                .current_dir(&checkout)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        symlink(&checkout, root.join("old")).unwrap();
        fs::write(root.join("heap.qcore"), HEAP).unwrap();
        fs::write(
            root.join("contracts.json"),
            json!({"schema_version": 1, "modules": ["heap.qcore"]}).to_string(),
        )
        .unwrap();
        fs::write(
            root.join("link-map.json"),
            json!({"schema_version": 1,
                   "premise": "every call to release resolves to its listed definition",
                   "symbols": [{"function": "release", "home_unit": "sample/src/home.c"}]})
            .to_string(),
        )
        .unwrap();
        let profile = root.join("profiles/sample");
        fs::create_dir_all(&profile).unwrap();
        let library = json!({
            "pin":"sample", "source_roots":[{"root":"src","selection":"top_level"}],
            "audited_files": [], "source_witnesses":[], "permitted_mechanisms":[],
            "operation_summaries":{}, "api_dispositions":{}, "accepted_builds":[],
            "required_evidence":[], "typed_import":null, "callback_summaries":{},
            "boolean_field_models":[], "provider":null,
        });
        fs::write(
            profile.join("profile.json"),
            json!({
                "schema_version":1, "source_set":"sample",
                "pins":{"sample":{"git_commit":commit,"clean_paths":[],"description":"canonical-path fixture"}},
                "libraries":{"sample":library},
                "resource_contract_manifest": "../../contracts.json",
                "provider_manifests":[],
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            profile.join("audit.json"),
            json!({"schema_version":1,"libraries":[{"library":"sample","label":null}]}).to_string(),
        )
        .unwrap();
        Self {
            root,
            driver: driver.to_path_buf(),
        }
    }

    /// One compile record: `directory` and `file` exactly as given.
    fn record(&self, directory: &Path, file: &str) -> Value {
        let object = Path::new(file).with_extension("o");
        json!({
            "directory": directory, "file": file,
            "arguments": [self.driver, "-std=c11", "-c", file, "-o", object],
        })
    }

    /// Write a database and run the configured route on it, with the link map.
    fn run(&self, name: &str, records: &[Value]) -> Output {
        fs::write(
            self.root.join(format!("{name}.json")),
            Value::Array(records.to_vec()).to_string(),
        )
        .unwrap();
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(&self.root)
            .env("QCORE_CROSS_UNIT_LINK_MAP", self.root.join("link-map.json"))
            .args([
                "current",
                "--checkout",
                "checkout",
                "--profile-directory",
                "profiles/sample",
                "--compile-database",
                &format!("{name}.json"),
                "--report",
                &format!("{name}/report.json"),
                "--jobs",
                "1",
            ])
            .output()
            .unwrap()
    }

    fn checkout(&self) -> PathBuf {
        self.root.join("checkout")
    }

    fn old(&self) -> PathBuf {
        self.root.join("old")
    }

    /// The report with every path the database spelled replaced, so two
    /// spellings of one run compare equal byte for byte where nothing but the
    /// spelling differs.
    fn report(&self, name: &str, spelled: &[(&Path, &Path)]) -> String {
        let mut text = fs::read_to_string(self.root.join(format!("{name}/report.json"))).unwrap();
        for (from, to) in spelled {
            text = text.replace(&*from.to_string_lossy(), &to.to_string_lossy());
        }
        text
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
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

fn refused(output: &Output, expected: &str) {
    assert!(
        !output.status.success(),
        "stdout:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "expected `{expected}` in:\n{stderr}"
    );
}

/// The keys under which a report publishes the spelled compile record or a
/// digest over it: the configuration record (spelled directory, file and
/// command, and the compile-task identity binding them) and the identity's
/// digest wherever a row or certificate carries it.
const SPELLING_KEYS: [&str; 2] = ["compile_configuration", "build_identity"];

/// The report's units with [`SPELLING_KEYS`] removed: everything a verdict
/// rests on, which must not depend on how the database spelled a path.
fn verdicts(report: &str) -> Value {
    fn strip(value: &mut Value) {
        match value {
            Value::Object(map) => {
                for key in SPELLING_KEYS {
                    map.remove(key);
                }
                map.values_mut().for_each(strip);
            }
            Value::Array(items) => items.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut report: Value = serde_json::from_str(report).unwrap();
    for unit in report["units"].as_array().unwrap() {
        assert!(unit["compile_configuration"].is_object(), "{unit}");
    }
    let mut units = report["units"].take();
    strip(&mut units);
    units
}

#[test]
fn a_database_spelled_through_a_symlinked_checkout_selects_the_same_units() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let real = fixture.checkout();
    let old = fixture.old();
    let records = |directory: &Path| {
        UNITS
            .iter()
            .map(|(path, _)| fixture.record(directory, &directory.join(path).to_string_lossy()))
            .collect::<Vec<_>>()
    };
    succeeded(&fixture.run("real", &records(&real)));
    // Before the card this route failed: `cross-unit home lies outside checkout`.
    succeeded(&fixture.run("old", &records(&old)));
    let real_report = fixture.report("real", &[]);
    let old_report = fixture.report("old", &[(&old, &real)]);
    assert_eq!(verdicts(&old_report), verdicts(&real_report));
    let units = verdicts(&real_report).as_array().unwrap().len();
    assert_eq!(units, 2, "{real_report}");
    // Published paths keep the database's spelling: the old spelling is in
    // the old run's configuration records, never rewritten to the real one.
    let raw = fixture.report("old", &[]);
    assert!(
        raw.contains(&*old.join("src/home.c").to_string_lossy()),
        "{raw}"
    );
}

#[test]
fn a_relative_file_resolves_against_its_symlinked_directory() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let real = fixture.checkout();
    let old = fixture.old();
    // {"directory": OLD/build, "file": "../src/X.c"}: resolved as Clang
    // would open it, through the symlink and then `..`.
    let relative = UNITS
        .iter()
        .map(|(path, _)| fixture.record(&old.join("build"), &format!("../{path}")))
        .collect::<Vec<_>>();
    let absolute = UNITS
        .iter()
        .map(|(path, _)| fixture.record(&real, &real.join(path).to_string_lossy()))
        .collect::<Vec<_>>();
    succeeded(&fixture.run("relative", &relative));
    succeeded(&fixture.run("absolute", &absolute));
    assert_eq!(
        verdicts(&fixture.report("relative", &[])),
        verdicts(&fixture.report("absolute", &[]))
    );
}

#[test]
fn a_record_resolving_outside_the_checkout_is_refused() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let outside = fixture.root.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("evil.c"), "void evil(void) {}\n").unwrap();
    // Spelled under the checkout's `src/`, resolving outside it.
    symlink(
        outside.join("evil.c"),
        fixture.checkout().join("src/evil.c"),
    )
    .unwrap();
    let old = fixture.old();
    let mut records = UNITS
        .iter()
        .map(|(path, _)| fixture.record(&old, &old.join(path).to_string_lossy()))
        .collect::<Vec<_>>();
    records.push(fixture.record(&old, &old.join("src/evil.c").to_string_lossy()));
    refused(
        &fixture.run("escape", &records),
        "source outside current checkout",
    );
    assert!(!fixture.root.join("escape/report.json").exists());
}

#[test]
fn a_dangling_record_is_refused_never_admitted() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let old = fixture.old();
    let mut records = UNITS
        .iter()
        .map(|(path, _)| fixture.record(&old, &old.join(path).to_string_lossy()))
        .collect::<Vec<_>>();
    // Located under the checkout's `src/` through its existing ancestor, so
    // it is selected and stays in the denominator; reading it refuses.
    records.push(fixture.record(&old, &old.join("src/missing.c").to_string_lossy()));
    refused(
        &fixture.run("dangling", &records),
        "No such file or directory",
    );
    assert!(!fixture.root.join("dangling/report.json").exists());
}

#[test]
fn two_spellings_of_one_source_are_refused_not_merged_or_counted_twice() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let real = fixture.checkout();
    let old = fixture.old();
    let mut records = UNITS
        .iter()
        .map(|(path, _)| fixture.record(&real, &real.join(path).to_string_lossy()))
        .collect::<Vec<_>>();
    records.push(fixture.record(&real, &old.join("src/home.c").to_string_lossy()));
    refused(&fixture.run("alias", &records), "input-path-alias");
    assert!(!fixture.root.join("alias/report.json").exists());
    // One spelling repeated is not an alias: the configured route keeps one
    // unit per distinct command, as before.
    let mut repeated = UNITS
        .iter()
        .map(|(path, _)| fixture.record(&real, &real.join(path).to_string_lossy()))
        .collect::<Vec<_>>();
    repeated.push(repeated[0].clone());
    succeeded(&fixture.run("repeated", &repeated));
    assert_eq!(
        verdicts(&fixture.report("repeated", &[]))
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
