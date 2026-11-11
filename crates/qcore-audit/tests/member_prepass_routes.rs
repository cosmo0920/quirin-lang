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

//! Card MEMBER-PREPASS-OTHER-ROUTES (gateway rule 12n, amended in place;
//! user decisions R1, F1, E1): the member-facts pre-pass on the current and
//! no-build routes, the checkpoint that carries the run's facts, and the
//! scope file's exclusions re-checked at run time.
//!
//! The fixture's `use_err` calls `serve_err`, whose contract invokes
//! `handle_s.conf.err_cb`. No unit stores into that member, so the run's set
//! is closed and empty and the invocation is the identity (E1): `use_err` is
//! `Safe` exactly when the unit decides over the run's set. Over the unit's
//! own set (no pre-pass) an empty set is refused.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_audit::checkpoint::Checkpoint;
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
            eprintln!("skipping Clang-dependent member pre-pass route test: {reason}");
            None
        }
    }
}

const RECORDS: &str = "\
typedef void log_fn(void *);
struct conf_s { log_fn *log_cb; log_fn *err_cb; int level; };
struct handle_s { int kind; struct conf_s conf; };
void serve_err(struct handle_s *rk);
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
";

/// The audited library's units.
fn units() -> Vec<(String, String)> {
    vec![
        (
            "lib/use.c".to_owned(),
            format!("{RECORDS}void use_err(struct handle_s *h) {{ serve_err(h); }}\n"),
        ),
        (
            "lib/other.c".to_owned(),
            format!("{RECORDS}void touch(struct handle_s *h) {{ h->kind = 1; }}\n"),
        ),
    ]
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

/// What the scope file says of `extra/aside.c`, a unit of the compile
/// database outside the audited library.
enum Aside {
    /// Not excluded (no scope file).
    Unscoped,
    /// Excluded with `tokens-absent` evidence.
    Excluded,
}

struct Fixture(PathBuf);

impl Fixture {
    fn new(driver: &Path, aside: &str, scope: Aside) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-member-prepass-routes-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let checkout = root.join("checkout");
        let mut files = units();
        files.push(("extra/aside.c".to_owned(), aside.to_owned()));
        for (path, text) in &files {
            let path = checkout.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        git(&checkout, &["init", "--quiet"]);
        git(&checkout, &["add", "lib", "extra"]);
        git(
            &checkout,
            &[
                "-c",
                "user.name=Prepass Fixture",
                "-c",
                "user.email=prepass@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Member pre-pass fixture",
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
            "pins":{"sample":{"git_commit":commit,"clean_paths":[],"description":"pre-pass fixture"}},
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
        if let Aside::Excluded = scope {
            fs::write(
                profile.join("member-facts-scope.json"),
                json!({
                    "schema_version": 2,
                    "evidence_revision": commit,
                    "meaning": "fixture",
                    "outside_program": [{
                        "fragment": "/extra/aside.c",
                        "why": "fixture: names no token of the pre-pass",
                        "evidence": {"kind": "tokens-absent"},
                    }],
                })
                .to_string(),
            )
            .unwrap();
        }
        fs::write(
            root.join("compile_commands.json"),
            json!(
                files
                    .iter()
                    .map(|(path, _)| json!({
                        "directory": checkout, "file": path,
                        "arguments": [driver, "-std=c11", "-c", path, "-o", path.replace(".c", ".o")],
                    }))
                    .collect::<Vec<_>>()
            )
            .to_string(),
        )
        .unwrap();
        Self(root)
    }

    fn current(&self, out: &str, extra: &[&str]) -> Output {
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
            .args(extra)
            .output()
            .unwrap()
    }

    fn checkpoint(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(&self.0)
            .arg("checkpoint")
            .args(args)
            .output()
            .unwrap()
    }

    fn json(&self, relative: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(relative)).unwrap()).unwrap()
    }

    fn bytes(&self, relative: &str) -> Vec<u8> {
        fs::read(self.0.join(relative)).unwrap()
    }
}

impl Drop for Fixture {
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

/// The checkpoints a run wrote, from its run record.
fn checkpoints(fixture: &Fixture, directory: &str) -> Vec<PathBuf> {
    fixture.json(&format!("{directory}/checkpoint-run.json"))["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|group| {
            fixture.0.join(format!(
                "{directory}/{}.qo",
                group["group"].as_str().unwrap()
            ))
        })
        .collect()
}

const PLAIN_ASIDE: &str = "int aside_counter;\n";

#[test]
fn the_current_route_decides_over_the_runs_set() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, PLAIN_ASIDE, Aside::Unscoped);
    let run = fixture.current("whole", &[]);
    succeeded(&run);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("member facts pre-pass: 3 unit(s)"),
        "{stdout}"
    );
    let report = fixture.json("whole/report.json");
    assert_eq!(status(&report, "use_err"), "Safe");
    let sidecar = fixture.json("whole/report.contracts.json");
    let members = &sidecar["callback_members"];
    assert_eq!(
        members["members"]["handle_s.conf.err_cb"],
        json!({"closed": true, "candidates": []})
    );
    assert_eq!(members["prepass"]["units"], json!(3));
    assert_eq!(members["prepass"]["unread"], json!({}));
    assert_eq!(members["prepass"]["scope"]["entries"], json!(0));
    assert_eq!(members["demoted"], json!([]));
}

#[test]
fn the_no_build_route_decides_over_the_runs_set() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, PLAIN_ASIDE, Aside::Unscoped);
    let run = Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
        .current_dir(&fixture.0)
        .env_remove("QCORE_BOUNDS_PROPERTY")
        .env_remove("QCORE_NULL_PROPERTY")
        .args([
            "contracts",
            "--source-root",
            "checkout",
            "lib",
            "--contract-manifest",
            "contracts.json",
            "--report",
            "nobuild/report.json",
        ])
        .output()
        .unwrap();
    succeeded(&run);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("member facts pre-pass: 2 unit(s)"),
        "{stdout}"
    );
    assert_eq!(
        status(&fixture.json("nobuild/report.json"), "use_err"),
        "Safe"
    );
    // A selected unit that does not preprocess under the assumptions and
    // names a token is unread: it opens every member, so nothing decides.
    fs::write(
        fixture.0.join("checkout/lib/broken.c"),
        "#include \"absent.h\"\nstruct handle_s *broken_handle;\n",
    )
    .unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
        .current_dir(&fixture.0)
        .env_remove("QCORE_BOUNDS_PROPERTY")
        .env_remove("QCORE_NULL_PROPERTY")
        .args([
            "contracts",
            "--source-root",
            "checkout",
            "lib",
            "--contract-manifest",
            "contracts.json",
            "--report",
            "nobuild-broken/report.json",
        ])
        .output()
        .unwrap();
    succeeded(&run);
    assert_ne!(
        status(&fixture.json("nobuild-broken/report.json"), "use_err"),
        "Safe"
    );
    let sidecar = fixture.json("nobuild-broken/report.contracts.json");
    let unread = sidecar["callback_members"]["prepass"]["unread"]
        .as_object()
        .unwrap();
    assert!(
        unread.keys().any(|unit| unit.ends_with("lib/broken.c")),
        "{unread:?}"
    );
}

#[test]
fn a_checkpoint_with_the_runs_facts_decides_and_one_without_refuses() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, PLAIN_ASIDE, Aside::Unscoped);
    succeeded(&fixture.current("whole", &[]));
    let checkpointed = fixture.current("ckpt", &["--checkpoint-dir", "qo"]);
    succeeded(&checkpointed);
    // The composed outputs are the whole run's, byte for byte: the member
    // invocation decided over the carried set.
    for name in ["report.json", "report.contracts.json"] {
        assert_eq!(
            fixture.bytes(&format!("ckpt/{name}")),
            fixture.bytes(&format!("whole/{name}")),
            "{name}"
        );
    }
    assert_eq!(status(&fixture.json("ckpt/report.json"), "use_err"), "Safe");
    let paths = checkpoints(&fixture, "qo");
    for path in &paths {
        let checkpoint = Checkpoint::from_bytes(&fs::read(path).unwrap()).unwrap();
        assert!(checkpoint.identity.run.member_facts_sha256.is_some());
        let carried = checkpoint
            .global
            .member_fold
            .as_ref()
            .unwrap()
            .to_json()
            .unwrap();
        assert_eq!(
            carried["run"]["closures"]["handle_s.conf.err_cb"],
            json!({"closed": true, "candidates": []})
        );
    }
    // A standalone composition of the same files decides the same.
    let mut args = vec!["compose"];
    let names = paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    args.extend(names.iter().map(String::as_str));
    args.extend(["--report", "composed/report.json"]);
    succeeded(&fixture.checkpoint(&args));
    assert_eq!(
        fixture.bytes("composed/report.json"),
        fixture.bytes("whole/report.json")
    );
    // The same checkpoints without the run's facts: refused.
    let stripped = fixture.0.join("stripped");
    fs::create_dir_all(&stripped).unwrap();
    let mut stripped_names = Vec::new();
    for path in &paths {
        let mut checkpoint = Checkpoint::from_bytes(&fs::read(path).unwrap()).unwrap();
        let mut carried = checkpoint
            .global
            .member_fold
            .as_ref()
            .unwrap()
            .to_json()
            .unwrap();
        carried["run"] = Value::Null;
        checkpoint.global.member_fold =
            Some(qcore_audit::checkpoint::BinaryValue::from_json(&carried).unwrap());
        checkpoint.identity.run.member_facts_sha256 = None;
        let target = stripped.join(path.file_name().unwrap());
        fs::write(&target, checkpoint.to_bytes().unwrap()).unwrap();
        stripped_names.push(target.display().to_string());
    }
    let mut args = vec!["compose"];
    args.extend(stripped_names.iter().map(String::as_str));
    args.extend(["--report", "stripped-out/report.json"]);
    let refused = fixture.checkpoint(&args);
    assert!(!refused.status.success());
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("cannot be composed without the run's member facts"),
        "{stderr}"
    );
    assert!(!fixture.0.join("stripped-out/report.json").exists());
}

#[test]
fn an_exclusion_that_still_holds_keeps_the_unit_outside() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, PLAIN_ASIDE, Aside::Excluded);
    succeeded(&fixture.current("held", &[]));
    assert_eq!(status(&fixture.json("held/report.json"), "use_err"), "Safe");
    let prepass = &fixture.json("held/report.contracts.json")["callback_members"]["prepass"];
    assert_eq!(prepass["scope"]["held"], json!(1));
    assert_eq!(prepass["scope"]["dropped"], json!({}));
    assert!(
        prepass["outside_program"]
            .as_object()
            .unwrap()
            .keys()
            .any(|unit| unit.ends_with("extra/aside.c")),
        "{prepass}"
    );
}

#[test]
fn an_exclusion_whose_unit_now_names_a_token_is_read() {
    let Some(clang) = clang() else { return };
    // The excluded unit now stores an uncontracted function into the member:
    // the exclusion no longer holds, the unit is read, its store joins the
    // run's set, and the invocation is no longer the identity.
    let aside = format!(
        "{RECORDS}void rogue(void *);\nvoid arm(struct handle_s *h) {{ h->conf.err_cb = rogue; }}\n"
    );
    let fixture = Fixture::new(&clang, &aside, Aside::Excluded);
    succeeded(&fixture.current("read", &[]));
    assert_ne!(status(&fixture.json("read/report.json"), "use_err"), "Safe");
    let members = &fixture.json("read/report.contracts.json")["callback_members"];
    assert_eq!(
        members["members"]["handle_s.conf.err_cb"],
        json!({"closed": true, "candidates": ["rogue"]})
    );
    let scope = &members["prepass"]["scope"];
    assert_eq!(scope["held"], json!(0));
    let dropped = scope["dropped"].as_object().unwrap();
    let (unit, why) = dropped.iter().next().unwrap();
    assert!(unit.ends_with("extra/aside.c"), "{unit}");
    assert!(
        why.as_str()
            .unwrap()
            .contains("names a token of the pre-pass"),
        "{why}"
    );
}

#[test]
fn an_exclusion_whose_unit_cannot_be_read_opens_every_member() {
    let Some(clang) = clang() else { return };
    // The excluded unit now includes a header nobody generates: its evidence
    // cannot hold, it is preprocessed like any other unit, its preprocessing
    // fails, and an unread unit opens every member.
    let aside = format!("#include \"absent_generated.h\"\n{RECORDS}int aside_counter;\n");
    let fixture = Fixture::new(&clang, &aside, Aside::Excluded);
    succeeded(&fixture.current("open", &[]));
    assert_ne!(status(&fixture.json("open/report.json"), "use_err"), "Safe");
    let members = &fixture.json("open/report.contracts.json")["callback_members"];
    assert_eq!(
        members["members"]["handle_s.conf.err_cb"]["closed"],
        json!(false)
    );
    let prepass = &members["prepass"];
    let dropped = prepass["scope"]["dropped"].as_object().unwrap();
    assert!(
        dropped.values().any(|why| why
            .as_str()
            .unwrap()
            .contains("`absent_generated.h` is not resolved")),
        "{dropped:?}"
    );
    assert!(
        prepass["unread"]
            .as_object()
            .unwrap()
            .keys()
            .any(|unit| unit.ends_with("extra/aside.c")),
        "{prepass}"
    );
}
