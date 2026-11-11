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

//! Card CALLBACK-MEMBER-NON-C-UNITS (gateway rule 12n, amended in place): an
//! unread C++ unit whose preprocessed text is the whole unit opens only the
//! members whose closure-graph tokens it names; every other unread unit (a
//! failed preprocessing, an assembly unit, a C++ unit reading a precompiled
//! header or module) opens every member.
//!
//! The fixture's library has two invoked members: `handle_s.conf.err_cb`
//! (through `serve_err`) and `other_s.cb` (through `serve_other`). No library
//! unit stores into either, so each closes empty and its invocation is the
//! identity (E1): `use_err` and `use_other` are `Safe` exactly when their
//! member's run set is closed. The units under `extra/` are in the compile
//! database but outside the audited library.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_contract::profile_loader::sha256_hex;
use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);
static CLANG: OnceLock<Result<PathBuf, String>> = OnceLock::new();

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
            eprintln!("skipping Clang-dependent non-C unit test: {reason}");
            None
        }
    }
}

const RECORDS: &str = "\
typedef void log_fn(void *);
struct conf_s { log_fn *log_cb; log_fn *err_cb; int level; };
struct handle_s { int kind; struct conf_s conf; };
struct other_s { log_fn *cb; };
void serve_err(struct handle_s *rk);
void serve_other(struct other_s *o);
";

const CONTRACTS: &str = "module contract;

extern fn serve_err(rk: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_revision(flexible);
    ensures normal resource_borrowed(rk);
    ensures normal resource_callback_member_invoked(rk, handle_s.conf.err_cb, rk);
    ensures normal resource_return_none();
};

extern fn serve_other(o: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_revision(flexible);
    ensures normal resource_borrowed(o);
    ensures normal resource_callback_member_invoked(o, other_s.cb, o);
    ensures normal resource_return_none();
};
";

/// The audited library's units.
fn units() -> Vec<(String, String)> {
    vec![(
        "lib/use.c".to_owned(),
        format!(
            "{RECORDS}void use_err(struct handle_s *h) {{ serve_err(h); }}\n\
             void use_other(struct other_s *o) {{ serve_other(o); }}\n"
        ),
    )]
}

/// A C++ unit that declares `other_s` and stores into its member, and never
/// names `handle_s`, `conf_s` or anything else of `handle_s.conf.err_cb`'s
/// closure graph.
const OTHER_ONLY: &str = "\
struct other_s { void (*cb)(void *); };
extern \"C\" void rogue(void *);
void arm(other_s *o) { o->cb = rogue; }
";

/// A C++ unit that declares the records of `handle_s.conf.err_cb` and stores
/// into it.
const STORES_ERR: &str = "\
typedef void log_fn(void *);
struct conf_s { log_fn *log_cb; log_fn *err_cb; int level; };
struct handle_s { int kind; conf_s conf; };
extern \"C\" void rogue(void *);
void arm(handle_s *h) { h->conf.err_cb = rogue; }
";

/// A C++ unit that only declares `handle_s` (a pointer parameter, no store).
const DECLARES_ERR: &str = "\
struct handle_s;
int count(handle_s *h);
";

struct Extra {
    path: &'static str,
    text: String,
    flags: Vec<String>,
}

fn extra(path: &'static str, text: &str) -> Extra {
    Extra {
        path,
        text: text.to_owned(),
        flags: Vec::new(),
    }
}

struct Fixture(PathBuf);

impl Fixture {
    fn new(driver: &Path, extras: &[Extra]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-member-non-c-units-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let checkout = root.join("checkout");
        let mut files = units()
            .into_iter()
            .map(|(path, text)| (path, text, Vec::new()))
            .collect::<Vec<_>>();
        files.extend(
            extras
                .iter()
                .map(|e| (e.path.to_owned(), e.text.clone(), e.flags.clone())),
        );
        for (path, text, _) in &files {
            let path = checkout.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        // A header a `-include` flag may name.
        fs::write(checkout.join("extra/pre.hpp"), "int pre_declared;\n").unwrap();
        git(&checkout, &["init", "--quiet"]);
        git(&checkout, &["add", "lib", "extra"]);
        git(
            &checkout,
            &[
                "-c",
                "user.name=Non-C Fixture",
                "-c",
                "user.email=non-c@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Non-C unit fixture",
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
        fs::create_dir_all(root.join("contracts")).unwrap();
        fs::write(root.join("contracts/cb.qcore"), CONTRACTS).unwrap();
        fs::write(
            root.join("contracts.json"),
            json!({"schema_version": 1, "modules": ["contracts/cb.qcore"]}).to_string(),
        )
        .unwrap();
        let profile = root.join("profiles/sample");
        fs::create_dir_all(&profile).unwrap();
        fs::write(profile.join("profile.json"), json!({
            "schema_version":1, "source_set":"sample",
            "pins":{"sample":{"git_commit":commit,"clean_paths":[],"description":"non-C unit fixture"}},
            "libraries":{"lib": {
                "pin":"sample", "source_roots":[{"root": "lib", "selection":"top_level"}],
                "audited_files": units().iter().map(|(path, text)| json!({"path": path, "sha256": sha256_hex(text.as_bytes())})).collect::<Vec<_>>(),
                "source_witnesses":[], "permitted_mechanisms":[], "operation_summaries":{},
                "api_dispositions":{}, "accepted_builds":[], "required_evidence":[], "typed_import":null,
                "callback_summaries":{}, "boolean_field_models":[], "provider":null,
            }},
            "resource_contract_manifest": "../../contracts.json",
            "provider_manifests":[],
        }).to_string()).unwrap();
        fs::write(
            profile.join("audit.json"),
            json!({"schema_version":1,"libraries":[{"library":"lib","label":null}]}).to_string(),
        )
        .unwrap();
        fs::write(
            root.join("compile_commands.json"),
            json!(
                files
                    .iter()
                    .map(|(path, _, flags)| {
                        let mut arguments = vec![driver.to_string_lossy().into_owned()];
                        if path.ends_with(".c") {
                            arguments.push("-std=c11".to_owned());
                        } else if path.ends_with(".cpp") {
                            arguments.push("-std=c++17".to_owned());
                        }
                        arguments.extend(flags.iter().cloned());
                        let object = format!("{path}.o");
                        arguments.extend(["-c".to_owned(), path.clone(), "-o".to_owned(), object]);
                        json!({"directory": checkout, "file": path, "arguments": arguments})
                    })
                    .collect::<Vec<_>>()
            )
            .to_string(),
        )
        .unwrap();
        Self(root)
    }

    fn current(&self, out: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(&self.0)
            .env_remove("QCORE_BOUNDS_PROPERTY")
            .env_remove("QCORE_NULL_PROPERTY")
            .env_remove("QCORE_MEMBER_PREPASS_SECTION")
            .args([
                "current",
                "--checkout",
                "checkout",
                "--profile-directory",
                "profiles/sample",
                "--compile-database",
                "compile_commands.json",
                "--report",
                &format!("{out}/report.json"),
                "--monkey-source-report",
                &format!("{out}/monkey-source.json"),
                "--jobs",
                "2",
            ])
            .output()
            .unwrap()
    }

    fn json(&self, relative: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(relative)).unwrap()).unwrap()
    }

    /// Run the current route; the report and the sidecar's member section.
    fn run(&self, out: &str) -> (Value, Value) {
        let output = self.current(out);
        assert!(
            output.status.success(),
            "stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (
            self.json(&format!("{out}/report.json")),
            self.json(&format!("{out}/report.contracts.json"))["callback_members"].clone(),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

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

/// The status of `function` in a report.
fn status(report: &Value, function: &str) -> String {
    report["units"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|unit| unit["functions"].as_array().unwrap())
        .find(|row| row["function"] == function)
        .unwrap_or_else(|| panic!("no row for {function}"))["status"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// The unit key of the pre-pass section that ends with `suffix`.
fn unit_key(object: &Value, suffix: &str) -> Option<String> {
    object
        .as_object()?
        .keys()
        .find(|unit| unit.ends_with(suffix))
        .cloned()
}

fn open_reasons(members: &Value, member: &str) -> String {
    assert_eq!(
        members["members"][member]["closed"],
        json!(false),
        "{members}"
    );
    members["members"][member]["open"].to_string()
}

#[test]
fn a_cxx_unit_that_never_names_the_members_graph_closes_it() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, &[extra("extra/other.cpp", OTHER_ONLY)]);
    let (report, members) = fixture.run("other");
    // The C++ unit names `other_s`, so it is unread; it names nothing of
    // `handle_s.conf.err_cb`'s graph, so that member closes empty.
    assert_eq!(
        members["members"]["handle_s.conf.err_cb"],
        json!({"closed": true, "candidates": []})
    );
    assert_eq!(status(&report, "use_err"), "Safe");
    // It stores into `other_s.cb`, which it names: that member stays open.
    let open = open_reasons(&members, "other_s.cb");
    assert!(
        open.contains("extra/other.cpp") && open.contains("naming `other_s`"),
        "{open}"
    );
    assert_ne!(status(&report, "use_other"), "Safe");
    let prepass = &members["prepass"];
    let unit = unit_key(&prepass["unread"], "extra/other.cpp").expect("unread");
    assert_eq!(prepass["non_c_units"]["scoped"][&unit], json!(["other_s"]));
    assert_eq!(
        prepass["non_c_units"]["opened_by"]["handle_s.conf.err_cb"],
        json!({})
    );
    assert!(
        prepass["non_c_units"]["opened_by"]["other_s.cb"]
            .get(&unit)
            .is_some(),
        "{prepass}"
    );
}

#[test]
fn a_cxx_unit_that_declares_the_record_keeps_the_member_open() {
    let Some(clang) = clang() else { return };
    // Stores into the member: open.
    let stores = Fixture::new(&clang, &[extra("extra/store.cpp", STORES_ERR)]);
    let (report, members) = stores.run("stores");
    let open = open_reasons(&members, "handle_s.conf.err_cb");
    assert!(open.contains("extra/store.cpp"), "{open}");
    assert_ne!(status(&report, "use_err"), "Safe");
    // The unit never names `other_s`: that member closes.
    assert_eq!(
        members["members"]["other_s.cb"],
        json!({"closed": true, "candidates": []})
    );
    assert_eq!(status(&report, "use_other"), "Safe");
    // Only names the record, with no store: still open (the reading reads
    // the text's tokens, not its stores).
    let declares = Fixture::new(&clang, &[extra("extra/declare.cpp", DECLARES_ERR)]);
    let (report, members) = declares.run("declares");
    let open = open_reasons(&members, "handle_s.conf.err_cb");
    assert!(
        open.contains("extra/declare.cpp") && open.contains("naming `handle_s`"),
        "{open}"
    );
    assert_ne!(status(&report, "use_err"), "Safe");
}

#[test]
fn a_cxx_unit_whose_preprocessing_fails_opens_every_member() {
    let Some(clang) = clang() else { return };
    let broken = format!("#include \"absent_generated.hpp\"\n{OTHER_ONLY}");
    let fixture = Fixture::new(&clang, &[extra("extra/broken.cpp", &broken)]);
    let (report, members) = fixture.run("broken");
    for member in ["handle_s.conf.err_cb", "other_s.cb"] {
        let open = open_reasons(&members, member);
        assert!(
            open.contains("extra/broken.cpp") && open.contains("its preprocessing failed"),
            "{open}"
        );
    }
    assert_ne!(status(&report, "use_err"), "Safe");
    assert_ne!(status(&report, "use_other"), "Safe");
    let prepass = &members["prepass"];
    assert_eq!(prepass["non_c_units"]["scoped"], json!({}));
}

#[test]
fn an_assembly_unit_naming_a_token_opens_every_member() {
    let Some(clang) = clang() else { return };
    // Assembly writes through offsets and symbols no record names: naming
    // `other_s` keeps `handle_s.conf.err_cb` open too.
    let fixture = Fixture::new(
        &clang,
        &[extra("extra/vm.S", "\t.globl other_s\nother_s:\n\tret\n")],
    );
    let (report, members) = fixture.run("asm");
    for member in ["handle_s.conf.err_cb", "other_s.cb"] {
        let open = open_reasons(&members, member);
        assert!(
            open.contains("extra/vm.S") && open.contains("a unit that is not C (`S`)"),
            "{open}"
        );
    }
    assert_ne!(status(&report, "use_err"), "Safe");
    assert_eq!(members["prepass"]["non_c_units"]["scoped"], json!({}));
}

#[test]
fn a_cxx_unit_whose_text_is_not_the_whole_unit_opens_every_member() {
    let Some(clang) = clang() else { return };
    // An `-include` may stand for a precompiled header.
    let mut included = extra("extra/other.cpp", OTHER_ONLY);
    included.flags = vec!["-include".to_owned(), "extra/pre.hpp".to_owned()];
    let fixture = Fixture::new(&clang, &[included]);
    let (report, members) = fixture.run("include");
    let open = open_reasons(&members, "handle_s.conf.err_cb");
    assert!(
        open.contains("extra/other.cpp")
            && open.contains("precompiled header or module (`-include`)"),
        "{open}"
    );
    assert_ne!(status(&report, "use_err"), "Safe");
    // A module import in the text.
    let imported = format!("{OTHER_ONLY}import other_module;\n");
    let fixture = Fixture::new(&clang, &[extra("extra/module.cpp", &imported)]);
    let (report, members) = fixture.run("module");
    let open = open_reasons(&members, "handle_s.conf.err_cb");
    assert!(
        open.contains("extra/module.cpp") && open.contains("declares or imports a module"),
        "{open}"
    );
    assert_ne!(status(&report, "use_err"), "Safe");
}
