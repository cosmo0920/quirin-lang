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

//! Card W45-FB-ENGINE-PROGRESS-MEASURE (gateway rule 44): the loop-measure
//! selection end to end through `quirin-audit current`. With
//! `QCORE_LOOP_MEASURE_PROPERTY` set, each projected function's loop facts are
//! written beside its custody status, every loop at its original line; the
//! custody report and its sidecars are byte-identical to the run without the
//! selection; a checkpointed run refuses the selection instead of omitting
//! the rows.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_contract::profile_loader::sha256_hex;
use serde_json::{Value, json};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn clang() -> Option<PathBuf> {
    match qcore_c_gateway::clang_command() {
        Ok(command) => Some(PathBuf::from(command.get_program())),
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent rule-44 audit test: {reason}");
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

const UNIT: &str = "unsigned char *alloc(void);\n\
void dispose(unsigned char *p);\n\
#define BATCH 4\n\
void counted(void) {\n\
    int i;\n\
    for (i = 0; i < BATCH; i++) {\n\
        unsigned char *p = alloc();\n\
        dispose(p);\n\
    }\n\
}\n\
void uncounted(int c) {\n\
    unsigned char *p = alloc();\n\
    while (c > 0) {\n\
        c--;\n\
    }\n\
    dispose(p);\n\
}\n";

const FILE: &str = "alpha/loops.c";

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

struct Fixture(PathBuf);

impl Fixture {
    fn new(driver: &Path) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-loop-measure-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let checkout = root.join("checkout");
        let path = checkout.join(FILE);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, UNIT).unwrap();
        git(&checkout, &["init", "--quiet"]);
        git(&checkout, &["add", "alpha"]);
        git(
            &checkout,
            &[
                "-c",
                "user.name=Loop Fixture",
                "-c",
                "user.email=loop@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Loop fixture",
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
        fs::write(root.join("contracts/heap.qcore"), HEAP).unwrap();
        fs::write(
            root.join("contracts.json"),
            json!({"schema_version": 1, "modules": ["contracts/heap.qcore"]}).to_string(),
        )
        .unwrap();
        let profile = root.join("profiles/sample");
        fs::create_dir_all(&profile).unwrap();
        fs::write(profile.join("profile.json"), json!({
            "schema_version":1, "source_set":"sample",
            "pins":{"sample":{"git_commit":commit,"clean_paths":[],"description":"loop fixture"}},
            "libraries":{"alpha": {
                "pin":"sample", "source_roots":[{"root": "alpha", "selection":"top_level"}],
                "audited_files": [{"path": FILE, "sha256": sha256_hex(UNIT.as_bytes())}],
                "source_witnesses":[], "permitted_mechanisms":[], "operation_summaries":{},
                "api_dispositions":{}, "accepted_builds":[], "required_evidence":[], "typed_import":null,
                "callback_summaries":{}, "boolean_field_models":[], "provider":null,
            }},
            "resource_contract_manifest": "../../contracts.json",
            "provider_manifests":[],
        }).to_string()).unwrap();
        fs::write(
            profile.join("audit.json"),
            json!({"schema_version":1,"libraries":[{"library":"alpha","label":null}]}).to_string(),
        )
        .unwrap();
        fs::write(
            root.join("compile_commands.json"),
            json!([{
                "directory": checkout, "file": FILE,
                "arguments": [driver, "-std=c11", "-c", FILE, "-o", "alpha/loops.o"],
            }])
            .to_string(),
        )
        .unwrap();
        Self(root)
    }

    fn run(&self, out: &str, extra: &[&str], rows: Option<&Path>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
        match rows {
            Some(rows) => command.env("QCORE_LOOP_MEASURE_PROPERTY", rows),
            None => command.env_remove("QCORE_LOOP_MEASURE_PROPERTY"),
        };
        command
            .env_remove("QCORE_BOUNDS_PROPERTY")
            .env_remove("QCORE_NULL_PROPERTY")
            .current_dir(&self.0)
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
            ])
            .args(extra)
            .output()
            .unwrap()
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }

    /// Every custody artifact of a run (its invocation record names the
    /// run's own output paths, so it is not compared).
    fn outputs(&self, out: &str) -> Vec<(String, Vec<u8>)> {
        [
            "report.json",
            "report.components.json",
            "report.contracts.json",
            "report.chunk-contracts.json",
            "report.source-effects.json",
            "monkey-source.json",
        ]
        .iter()
        .map(|name| {
            (
                (*name).to_owned(),
                fs::read(self.0.join(out).join(name)).unwrap(),
            )
        })
        .collect()
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

#[test]
fn the_selection_writes_facts_beside_an_unchanged_custody_report() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let rows_path = fixture.path("loops.jsonl");
    succeeded(&fixture.run("selected", &["--jobs", "1"], Some(&rows_path)));
    succeeded(&fixture.run("plain", &["--jobs", "1"], None));
    // Every custody artifact is byte-identical with and without the selection.
    let (selected, plain) = (fixture.outputs("selected"), fixture.outputs("plain"));
    assert_eq!(
        selected.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        plain.iter().map(|(name, _)| name).collect::<Vec<_>>()
    );
    for ((name, left), (_, right)) in selected.iter().zip(&plain) {
        assert!(left == right, "{name} differs");
    }
    let rows = fs::read_to_string(&rows_path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    let row = |name: &str| {
        rows.iter()
            .find(|row| row["function"] == name)
            .unwrap_or_else(|| panic!("no row for {name}: {rows:?}"))
    };
    let counted = row("counted");
    assert_eq!(counted["library"], "alpha");
    assert_eq!(counted["file"], FILE);
    assert!(counted["custody"].is_string(), "{counted}");
    let fact = &counted["loops"][0];
    assert_eq!(fact["origin"], format!("{FILE}:6"));
    assert_eq!(fact["measure"]["bound_kind"], "Constant");
    assert_eq!(fact["measure"]["max_iterations"], 4);
    assert!(fact["refusal"].is_null());
    let uncounted = row("uncounted");
    assert_eq!(uncounted["loops"][0]["refusal"], "no-counter");
    assert_eq!(uncounted["loops"][0]["origin"], format!("{FILE}:13"));
    let summary: Value =
        serde_json::from_slice(&fs::read(fixture.path("loops.summary.json")).unwrap()).unwrap();
    assert_eq!(summary["property"], "loop-measure.v1");
    assert_eq!(summary["loops"]["measured"], 1);
    assert_eq!(summary["loops"]["refused"], 1);
}

#[test]
fn a_checkpointed_run_refuses_the_loop_measure_selection() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let rows = fixture.path("loops.jsonl");
    let output = fixture.run("qo", &["--checkpoint-dir", "qo"], Some(&rows));
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("checkpoints refuse the loop-measure selection"),
        "{stderr}"
    );
    assert!(!rows.exists(), "no partial rows are written");
}
