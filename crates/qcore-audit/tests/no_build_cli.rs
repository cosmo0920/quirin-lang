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

//! Card W44-CONTRACTS-ONLY-NO-BUILD: `quirin-audit contracts`, the
//! contracts-only audit that reads no compilation database or prepared build,
//! end to end on the command line, in JSON and both.

use std::fs;
use std::path::PathBuf;
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
            eprintln!("skipping Clang-dependent no-build test: {reason}");
            None
        }
    }
}

// Card W45-CONTRACT-REVISION-MODE: a synthetic API with no source to pin
// against, so its contracts bind by name (`revision flexible`).
const HEAP: &str = "module contract;\n\
extern fn alloc() -> ptr<mut, u8>\n\
contract {\n\
    requires resource_contract_v1;\n\
    requires resource_revision(flexible);\n\
    ensures normal resource_return_owned(nonnull);\n\
};\n\
extern fn dispose(p: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    requires resource_revision(flexible);\n\
    ensures normal resource_released(p, nonnull);\n\
    ensures normal resource_return_none();\n\
};\n";

const PROTOTYPES: &str = "unsigned char *alloc(void);\nvoid dispose(unsigned char *p);\n";

/// The fixture tree: units that preprocess with nothing, one that needs
/// `-I inc`, one whose release is selected by `-D RELEASE_IT`, and several
/// more so a parallel fold has distinct rows to order.
fn units() -> Vec<(&'static str, String)> {
    let mut units = vec![
        (
            "tree/plain.c",
            format!("{PROTOTYPES}void plain(void) {{ unsigned char *p = alloc(); dispose(p); }}\n"),
        ),
        (
            "tree/needs_include.c",
            "#include \"heap_api.h\"\nvoid included(void) { unsigned char *p = alloc(); dispose(p); }\n"
                .to_owned(),
        ),
        (
            "tree/configured.c",
            format!(
                "{PROTOTYPES}void configured(void) {{ unsigned char *p = alloc();\n\
                 #ifdef RELEASE_IT\n dispose(p);\n#endif\n}}\n"
            ),
        ),
    ];
    for index in 0..5 {
        units.push((
            ["tree/sub/a.c", "tree/sub/b.c", "tree/sub/c.c", "tree/sub/d.c", "tree/sub/e.c"][index],
            format!(
                "{PROTOTYPES}void extra_{index}(int c) {{ unsigned char *p = alloc(); if (c > {index}) dispose(p); }}\n"
            ),
        ));
    }
    units
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-no-build-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("tree/sub")).unwrap();
        fs::create_dir_all(root.join("tree/.hidden")).unwrap();
        fs::create_dir_all(root.join("inc")).unwrap();
        fs::create_dir_all(root.join("empty")).unwrap();
        for (path, text) in units() {
            fs::write(root.join(path), text).unwrap();
        }
        // A hidden directory's unit is never selected by a walk.
        fs::write(root.join("tree/.hidden/skipped.c"), "int skipped;\n").unwrap();
        fs::write(root.join("inc/heap_api.h"), PROTOTYPES).unwrap();
        let module = root.join("providers/quirin-provider-alpha/contracts/alpha/heap.qcore");
        fs::create_dir_all(module.parent().unwrap()).unwrap();
        fs::write(module, HEAP).unwrap();
        fs::write(
            root.join("providers/contract-load-path.json"),
            serde_json::to_string_pretty(&json!({"schema_version": 1, "crates": [
                {"name": "quirin-provider-alpha", "namespace": "alpha", "version": "0.1.0",
                 "contracts": "quirin-provider-alpha/contracts", "dependencies": [],
                 "modules": ["alpha/heap.qcore"], "published": {},
                 "resource_view": ["alpha/heap.qcore"]},
            ]}))
            .unwrap(),
        )
        .unwrap();
        Self(root)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(&self.0)
            .arg("contracts")
            .args(args)
            .output()
            .unwrap()
    }

    fn config(&self, name: &str, value: &Value) -> String {
        fs::write(
            self.0.join(name),
            serde_json::to_string_pretty(value).unwrap(),
        )
        .unwrap();
        name.to_owned()
    }

    fn read(&self, path: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(path)).unwrap()).unwrap()
    }

    fn bytes(&self, path: &str) -> Vec<u8> {
        fs::read(self.0.join(path)).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn succeeded(output: &Output) -> String {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn refused(output: &Output, expected: &str) {
    assert!(
        !output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "expected `{expected}` in:\n{stderr}"
    );
}

fn unit<'a>(report: &'a Value, file: &str) -> &'a Value {
    report["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["file"] == file)
        .unwrap_or_else(|| panic!("no row for {file}"))
}

#[test]
fn units_preprocess_with_no_settings_and_missing_includes_are_unknown_with_a_reason() {
    let Some(_) = clang() else { return };
    let fixture = Fixture::new();
    let stdout = succeeded(&fixture.run(&[
        "--source-root",
        "tree",
        "--load-path",
        "providers/contract-load-path.json",
        "--report",
        "out/report.json",
        "--jobs",
        "1",
    ]));
    assert!(
        stdout.contains("not prepared: missing-include=1"),
        "{stdout}"
    );
    let report = fixture.read("out/report.json");
    // Every walked unit has a row: none is dropped, and the hidden one is not walked.
    let files = report["units"]
        .as_array()
        .unwrap()
        .iter()
        .map(|unit| unit["file"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        files,
        [
            "configured.c",
            "needs_include.c",
            "plain.c",
            "sub/a.c",
            "sub/b.c",
            "sub/c.c",
            "sub/d.c",
            "sub/e.c"
        ]
    );
    // A unit that preprocesses with no settings is analysed.
    let plain = unit(&report, "plain.c");
    assert_eq!(plain["status"], "Safe");
    assert_eq!(plain["assumptions"]["outcome"], "analysed");
    assert_eq!(plain["assumptions"]["reason"], Value::Null);
    // The missing include is Unknown with its reason code and first diagnostic.
    let missing = unit(&report, "needs_include.c");
    assert_eq!(missing["status"], "Unknown");
    assert_eq!(missing["reason_code"], "environment-unavailable");
    assert_eq!(missing["assumptions"]["outcome"], "not-prepared");
    assert_eq!(missing["assumptions"]["reason"], "missing-include");
    let first = missing["assumptions"]["first_diagnostic"].as_str().unwrap();
    assert!(first.contains("'heap_api.h' file not found"), "{first}");
    assert!(
        missing["reason"]
            .as_str()
            .unwrap()
            .contains("(missing-include)")
    );
    assert_eq!(missing["source_functions"], json!(["included"]));
    assert!(
        missing["compile_configuration"]["compile_task_identity"]["preprocessing_error"]
            .as_str()
            .unwrap()
            .contains("file not found")
    );
    // Without RELEASE_IT the release is not compiled: a provisional leak.
    assert_eq!(unit(&report, "configured.c")["status"], "Unsafe");
    let sidecar = fixture.read("out/report.contracts.json");
    assert_eq!(
        sidecar["outcomes"]["not_prepared"],
        json!({"missing-include": 1})
    );
    assert_eq!(sidecar["outcomes"]["units"], 8);
    assert_eq!(sidecar["selection"]["hidden_directories_skipped"], 1);
}

#[test]
fn an_include_directory_turns_the_missing_include_into_an_analysed_unit() {
    let Some(_) = clang() else { return };
    let fixture = Fixture::new();
    for (out, extra) in [("without", vec![]), ("with", vec!["-I", "inc"])] {
        let report = format!("{out}/report.json");
        let mut args = vec![
            "--source-root",
            "tree",
            "needs_include.c",
            "--load-path",
            "providers/contract-load-path.json",
            "--report",
            report.as_str(),
        ];
        args.extend(extra);
        succeeded(&fixture.run(&args));
    }
    let without = fixture.read("without/report.json");
    let with = fixture.read("with/report.json");
    assert_eq!(unit(&without, "needs_include.c")["status"], "Unknown");
    assert_eq!(
        unit(&without, "needs_include.c")["assumptions"]["reason"],
        "missing-include"
    );
    let analysed = unit(&with, "needs_include.c");
    assert_eq!(analysed["status"], "Safe");
    let include = fixture.0.join("inc").canonicalize().unwrap();
    assert_eq!(analysed["assumptions"]["include_dirs"], json!([include]));
    assert_eq!(analysed["library"], ".");
}

#[test]
fn a_definition_changes_the_analysed_code_and_the_bound_assumptions() {
    let Some(_) = clang() else { return };
    let fixture = Fixture::new();
    for (out, extra) in [("plain", vec![]), ("defined", vec!["-DRELEASE_IT"])] {
        let report = format!("{out}/report.json");
        let mut args = vec![
            "--source-root",
            "tree",
            "configured.c",
            "--load-path",
            "providers/contract-load-path.json",
            "--report",
            report.as_str(),
        ];
        args.extend(extra);
        succeeded(&fixture.run(&args));
    }
    let plain = fixture.read("plain/report.json");
    let defined = fixture.read("defined/report.json");
    let (plain, defined) = (unit(&plain, "configured.c"), unit(&defined, "configured.c"));
    assert_eq!(plain["status"], "Unsafe");
    assert_eq!(defined["status"], "Safe");
    assert_eq!(defined["assumptions"]["defines"], json!(["RELEASE_IT"]));
    let identity = |row: &Value| row["compile_configuration"]["compile_task_identity"].clone();
    // Different code was analysed, and each verdict is bound to its own
    // assumption record: the source-profile dependency differs.
    assert_ne!(
        identity(plain)["preprocessed_source_sha256"],
        identity(defined)["preprocessed_source_sha256"]
    );
    assert_ne!(
        identity(plain)["dependencies"]["source_profile"],
        identity(defined)["dependencies"]["source_profile"]
    );
    assert_eq!(
        identity(defined)["dependencies"]["source_profile"],
        fixture.read("defined/report.contracts.json")["source_profile_sha256"]
    );
}

#[test]
fn command_line_configuration_and_both_forms_agree_and_conflicts_refuse() {
    let Some(_) = clang() else { return };
    let fixture = Fixture::new();
    succeeded(&fixture.run(&[
        "--source-root",
        "tree",
        "plain.c",
        "needs_include.c",
        "-I",
        "inc",
        "-D",
        "RELEASE_IT",
        "--std",
        "gnu11",
        "--load-path",
        "providers/contract-load-path.json",
        "--report",
        "cli/report.json",
        "--jobs",
        "1",
    ]));
    let config = fixture.config(
        "contracts.json",
        &json!({
            "schema_version": 1, "mode": "contracts-only", "build": "none",
            "source_root": "tree", "sources": ["plain.c", "needs_include.c"],
            "include_dirs": ["inc"], "defines": ["RELEASE_IT"], "std": "gnu11",
            "load_path": "providers/contract-load-path.json",
            "report": "config/report.json", "jobs": 2,
        }),
    );
    succeeded(&fixture.run(&["--config", &config]));
    // Both forms: equal declarations agree; the flag's job count wins.
    let both = fixture.config(
        "both.json",
        &json!({
            "schema_version": 1, "mode": "contracts-only", "build": "none",
            "include_dirs": ["inc"], "defines": ["RELEASE_IT"], "std": "gnu11", "jobs": 2,
        }),
    );
    succeeded(&fixture.run(&[
        "--config",
        &both,
        "--source-root",
        "tree",
        "plain.c",
        "needs_include.c",
        "-Iinc",
        "-DRELEASE_IT",
        "--std=gnu11",
        "--load-path",
        "providers/contract-load-path.json",
        "--report",
        "both/report.json",
        "--jobs",
        "1",
    ]));
    let report = fixture.bytes("cli/report.json");
    assert_eq!(fixture.bytes("config/report.json"), report);
    assert_eq!(fixture.bytes("both/report.json"), report);
    let sidecar = fixture.bytes("cli/report.contracts.json");
    assert_eq!(fixture.bytes("config/report.contracts.json"), sidecar);
    assert_eq!(fixture.bytes("both/report.contracts.json"), sidecar);
    let read = |out: &str| fixture.read(&format!("{out}/report.invocation.json"));
    assert_eq!(
        read("cli")["jobs"],
        json!({"requested": 1, "effective": 1, "source": "command_line"})
    );
    assert_eq!(
        read("config")["jobs"],
        json!({"requested": 2, "effective": 2, "source": "configuration"})
    );
    assert_eq!(
        read("both")["jobs"],
        json!({"requested": 1, "effective": 1, "source": "command_line"})
    );
    assert_eq!(read("both")["std_form"], "command_line+configuration");
    assert_eq!(
        read("both")["assumed_configuration"]["std_source"],
        "declared"
    );
    assert_eq!(read("cli")["build"], "none");
    assert_eq!(read("cli")["evidence"], "contracts-only-no-build");

    // Disagreeing declarations refuse; nothing is merged or overridden.
    refused(
        &fixture.run(&["--config", &both, "--source-root", "tree", "-I", "tree"]),
        "`include_dirs` conflicts",
    );
    refused(
        &fixture.run(&[
            "--config",
            &both,
            "--source-root",
            "tree",
            "-Iinc",
            "-DOTHER",
        ]),
        "`defines` conflicts",
    );
    refused(
        &fixture.run(&[
            "--config",
            &both,
            "--source-root",
            "tree",
            "-Iinc",
            "-DRELEASE_IT",
            "--std",
            "c11",
        ]),
        "`std` conflicts",
    );
    // The configuration names its route and refuses build inputs.
    for (name, value, expected) in [
        (
            "m.json",
            json!({"schema_version": 1, "mode": "configured", "build": "none"}),
            "`mode` must be `contracts-only`",
        ),
        (
            "b.json",
            json!({"schema_version": 1, "mode": "contracts-only", "build": "cmake"}),
            "`build` must be `none`",
        ),
        (
            "d.json",
            json!({"schema_version": 1, "mode": "contracts-only", "build": "none", "compile_database": "c.json"}),
            "reads no compilation database",
        ),
        (
            "p.json",
            json!({"schema_version": 1, "mode": "contracts-only", "build": "none", "packages": []}),
            "no process packages",
        ),
    ] {
        let config = fixture.config(name, &value);
        refused(
            &fixture.run(&["--config", &config, "--source-root", "tree"]),
            expected,
        );
    }
    refused(
        &fixture.run(&["--compile-database", "x.json"]),
        "unknown argument",
    );
}

#[test]
fn selections_that_name_nothing_or_leave_the_root_refuse() {
    let Some(_) = clang() else { return };
    let fixture = Fixture::new();
    refused(
        &fixture.run(&["--source-root", ".", "empty"]),
        "selects no `*.c` unit",
    );
    refused(
        &fixture.run(&["--source-root", "tree", "../inc/heap_api.h"]),
        "outside the source root",
    );
    refused(
        &fixture.run(&["--source-root", "tree", "-I", "missing"]),
        "include directory `",
    );
    // With no load path and no manifest, the working directory's default
    // load path is read, and every crate's resource view is loaded.
    succeeded(&fixture.run(&[
        "--source-root",
        "tree",
        "plain.c",
        "--report",
        "default/report.json",
    ]));
    let invocation = fixture.read("default/report.invocation.json");
    assert_eq!(invocation["contracts"]["load_path_source"], "default");
    assert_eq!(invocation["contracts"]["selection"], "all-loaded");
}

#[test]
fn one_and_sixteen_jobs_write_identical_outputs() {
    let Some(_) = clang() else { return };
    let fixture = Fixture::new();
    let mut logs = Vec::new();
    for jobs in ["1", "16"] {
        let report = format!("jobs{jobs}/report.json");
        let stdout = succeeded(&fixture.run(&[
            "--source-root",
            "tree",
            "--load-path",
            "providers/contract-load-path.json",
            "--report",
            &report,
            "--jobs",
            jobs,
        ]));
        logs.push(
            stdout
                .lines()
                .filter(|line| !line.starts_with("preparation:"))
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(logs[0], logs[1]);
    for artifact in ["report.json", "report.contracts.json"] {
        assert_eq!(
            fixture.bytes(&format!("jobs1/{artifact}")),
            fixture.bytes(&format!("jobs16/{artifact}")),
            "{artifact}"
        );
    }
}

#[test]
fn the_assumption_record_travels_with_every_verdict() {
    let Some(_) = clang() else { return };
    let fixture = Fixture::new();
    succeeded(&fixture.run(&[
        "--source-root",
        "tree",
        "plain.c",
        "sub",
        "-D",
        "LEVEL=2",
        "--load-path",
        "providers/contract-load-path.json",
        "--report",
        "out/report.json",
    ]));
    let report = fixture.read("out/report.json");
    let sidecar = fixture.read("out/report.contracts.json");
    let tree = fixture.0.join("tree").canonicalize().unwrap();
    // The report's evidence block names the route and its scope.
    let evidence = &report["evidence"];
    assert_eq!(evidence["evidence"], "contracts-only-no-build");
    assert_eq!(evidence["mode"], "contracts-only");
    assert_eq!(evidence["build"], "none");
    assert!(
        evidence["equivalence_to_build_configured"]
            .as_str()
            .unwrap()
            .starts_with("none")
    );
    assert_eq!(evidence["assumed_configuration"]["std"], "gnu17");
    assert_eq!(evidence["assumed_configuration"]["std_source"], "default");
    assert_eq!(
        evidence["assumed_configuration"]["defines"],
        json!(["LEVEL=2"])
    );
    // Every row carries its own effective settings and the verdict scope.
    for row in report["units"].as_array().unwrap() {
        let assumptions = &row["assumptions"];
        assert_eq!(assumptions["evidence"], "contracts-only-no-build");
        assert_eq!(assumptions["build"], "none");
        assert_eq!(assumptions["std"], "gnu17");
        assert_eq!(assumptions["defines"], json!(["LEVEL=2"]));
        assert!(
            assumptions["verdict_scope"]
                .as_str()
                .unwrap()
                .starts_with("relative to the recorded assumptions")
        );
        let file = row["file"].as_str().unwrap();
        let directory = tree.join(file).parent().unwrap().to_path_buf();
        assert_eq!(assumptions["quote_include_directory"], json!(directory));
        // The compile task is bound to the assumption record, so its
        // certificate cannot be admitted for a configured task.
        assert_eq!(
            row["compile_configuration"]["compile_task_identity"]["dependencies"]["source_profile"],
            sidecar["source_profile_sha256"]
        );
        assert_eq!(row["compile_configuration"]["build"], "none");
    }
    // The sidecar lists every file with its settings, and the rows' libraries
    // are the selection arguments.
    let files = sidecar["files"].as_array().unwrap();
    assert_eq!(files.len(), 6);
    assert_eq!(files[0]["file"], "plain.c");
    assert_eq!(files[0]["library"], ".");
    assert_eq!(files[1]["library"], "sub");
    assert_eq!(files[1]["defines"], json!(["LEVEL=2"]));
    assert_eq!(files[1]["outcome"], "analysed");
    assert!(files[1]["raw_source_sha256"].as_str().unwrap().len() == 64);
    assert_eq!(sidecar["evidence"], "contracts-only-no-build");
    assert_eq!(sidecar["selection"]["sources"], json!(["plain.c", "sub"]));
    assert_eq!(sidecar["provider_selection"]["selection"], "all-loaded");
}

/// Card RULE-SWITCH-ENTRY-POINTS: `contracts` with a report records the
/// effective switch list in every component (`rule_set.off`), and without a
/// report, where nothing would record it, refuses a non-empty list before any
/// unit is analysed. A refused list is refused either way.
#[test]
fn a_switch_list_is_recorded_in_every_component_or_refused_without_a_report() {
    let Some(_) = clang() else { return };
    let fixture = Fixture::new();
    let run = |list: &str, report: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
        command.current_dir(&fixture.0).args([
            "contracts",
            "--source-root",
            "tree",
            "--load-path",
            "providers/contract-load-path.json",
            "--jobs",
            "1",
        ]);
        if report {
            command.args(["--report", "out/report.json"]);
        }
        command
            .env("QCORE_GATEWAY_RULES_OFF", list)
            .output()
            .unwrap()
    };
    let rule_sets = || {
        let sidecar: qcore_schemas::composition::ComponentSidecar =
            serde_json::from_value(fixture.read("out/report.components.json")).unwrap();
        sidecar
            .contribution
            .components()
            .map(|component| component.provenance.rule_set.off.clone())
            .collect::<std::collections::BTreeSet<_>>()
    };
    succeeded(&run("35,10s", true));
    let switched = std::collections::BTreeSet::from(["10s".to_owned(), "35".to_owned()]);
    assert_eq!(rule_sets(), std::collections::BTreeSet::from([switched]));
    succeeded(&run("", true));
    assert_eq!(
        rule_sets(),
        std::collections::BTreeSet::from([std::collections::BTreeSet::new()])
    );
    let unreported = run("35", false);
    refused(
        &unreported,
        "quirin-audit contracts without a report does not record the gateway rule set",
    );
    assert!(unreported.stdout.is_empty(), "nothing was analysed");
    succeeded(&run("", false));
    refused(&run("35,no-such-rule", true), "no-such-rule");
}
