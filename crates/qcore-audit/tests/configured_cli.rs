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

use qcore_contract::profile_loader::sha256_hex;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::{
    Once, OnceLock,
    atomic::{AtomicUsize, Ordering},
};

// The Clang wrapper is written by a separate process, so a sibling test's concurrent
// spawn cannot make its `execve` fail with `ETXTBSY`
// (`tests/support/process_fixture.rs`). Data files stay `fs::write`.
#[path = "../../../tests/support/process_fixture.rs"]
mod process_fixture;

static NEXT: AtomicUsize = AtomicUsize::new(0);
static CLANG_AVAILABILITY: OnceLock<Result<(), String>> = OnceLock::new();
static CLANG_WARNING: Once = Once::new();

fn clang_available() -> bool {
    let availability =
        CLANG_AVAILABILITY.get_or_init(|| qcore_c_gateway::clang_command().map(|_| ()));
    match availability {
        Ok(()) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            CLANG_WARNING.call_once(|| {
                eprintln!("skipping Clang-dependent configured-audit tests: {reason}");
            });
            false
        }
    }
}

struct Fixture(PathBuf);
impl Fixture {
    fn new(source: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-config-audit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("checkout/src")).unwrap();
        fs::write(root.join("checkout/src/unit.c"), source).unwrap();
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "src/unit.c"],
            vec![
                "-c",
                "user.name=Audit Fixture",
                "-c",
                "user.email=audit@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Pinned generic fixture",
            ],
        ] {
            let output = Command::new("git")
                .current_dir(root.join("checkout"))
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let commit = Command::new("git")
            .current_dir(root.join("checkout"))
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        let commit = String::from_utf8(commit.stdout).unwrap().trim().to_owned();
        fs::create_dir_all(root.join("profiles/sample")).unwrap();
        fs::write(
            root.join("alloc.qcore"),
            include_str!("fixtures/alloc_acquire.qv"),
        )
        .unwrap();
        fs::write(
            root.join("contracts.json"),
            json!({"schema_version":1,"modules":["alloc.qcore"]}).to_string(),
        )
        .unwrap();
        let library = json!({
            "pin":"sample", "source_roots":[{"root":"src","selection":"top_level"}],
            "audited_files":[{"path":"src/unit.c","sha256":sha256_hex(source.as_bytes())}],
            "source_witnesses":[], "permitted_mechanisms":[], "operation_summaries":{},
            "api_dispositions":{}, "accepted_builds":[], "required_evidence":[], "typed_import":null,
            "callback_summaries":{}, "boolean_field_models":[], "provider":null,
        });
        fs::write(root.join("profiles/sample/profile.json"), json!({
            "schema_version":1,"source_set":"sample", "pins":{"sample":{"git_commit":commit,"clean_paths":[],"description":"generic fixture"}},
            "libraries":{"sample":library}, "resource_contract_manifest":"../../contracts.json", "provider_manifests":[],
        }).to_string()).unwrap();
        fs::write(root.join("compile_commands.json"), json!([{
            "directory": root.join("checkout"), "file":"src/unit.c", "arguments":["cc", "-std=c11", "-c", "src/unit.c", "-o", "unit.o"]
        }]).to_string()).unwrap();
        fs::write(root.join("audit.json"), json!({
            "schema_version":1,"profile_directory":"profiles/sample","checkout":"checkout",
            "compile_database":"compile_commands.json","libraries":["sample"],"report":"output/report.json",
        }).to_string()).unwrap();
        Self(root)
    }
    fn run(&self) -> Output {
        self.run_in(&self.0.clone())
    }
    /// Run the audit from an explicit working directory. A unit's meaning must
    /// not depend on where the audit happens to be started.
    fn run_in(&self, directory: &std::path::Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(directory)
            .arg(self.0.join("audit.json"))
            .output()
            .unwrap()
    }

    fn mutate(&self, path: &str, change: impl FnOnce(&mut Value)) {
        let path = self.0.join(path);
        let mut value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        change(&mut value);
        fs::write(path, value.to_string()).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn one_configured_unit_runs_each_preparation_stage_once() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("void subject(void) { return; }\n");
    let real = qcore_c_gateway::clang_command()
        .unwrap()
        .get_program()
        .to_owned();
    let wrapper = fixture.0.join("counting-clang");
    let log = fixture.0.join("clang-invocations.log");
    process_fixture::write_executable_mode(
        &wrapper,
        b"#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$QCORE_TEST_LOG\"\nexec \"$QCORE_TEST_REAL_CLANG\" \"$@\"\n",
        0o755,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
        .current_dir(&fixture.0)
        .arg(fixture.0.join("audit.json"))
        .env("QCORE_CLANG", &wrapper)
        .env("QCORE_TEST_LOG", &log)
        .env("QCORE_TEST_REAL_CLANG", &real)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let invocations = fs::read_to_string(&log).unwrap();
    let actions = invocations
        .lines()
        .filter(|line| !line.contains("--version") && !line.contains("-dumpmachine"))
        .collect::<Vec<_>>();
    let plain = actions
        .iter()
        .filter(|line| line.contains(" -E ") && line.contains(" -P "))
        .count();
    let marked = actions
        .iter()
        .filter(|line| line.contains(" -E ") && !line.contains(" -P "))
        .count();
    let syntax = actions
        .iter()
        .filter(|line| line.contains(" -fsyntax-only "))
        .count();
    assert_eq!((plain, syntax, marked), (1, 1, 1), "{actions:#?}");
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("preparation: 1 unit(s), 3 Clang invocation(s)"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Card RULE-SWITCH-ENTRY-POINTS: the configuration route records the
/// effective switch list in every component (`rule_set.off`) and in
/// `invocation.json`, and nothing when the list is empty.
#[test]
fn the_configuration_route_records_its_switch_list() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("void subject(void) { return; }\n");
    for (list, expected) in [("10s, 35", vec!["10s", "35"]), ("", vec![])] {
        let output = Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(&fixture.0)
            .arg(fixture.0.join("audit.json"))
            .env("QCORE_GATEWAY_RULES_OFF", list)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let expected = expected
            .into_iter()
            .map(str::to_owned)
            .collect::<std::collections::BTreeSet<_>>();
        let sidecar: qcore_schemas::composition::ComponentSidecar = serde_json::from_slice(
            &fs::read(fixture.0.join("output/report.components.json")).unwrap(),
        )
        .unwrap();
        let recorded = sidecar
            .contribution
            .components()
            .map(|component| component.provenance.rule_set.off.clone())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            recorded,
            std::collections::BTreeSet::from([expected.clone()])
        );
        let invocation: Value = serde_json::from_slice(
            &fs::read(fixture.0.join("output/report.invocation.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(invocation["gateway_rules_off"], json!(expected));
        assert_eq!(invocation["environment"]["QCORE_GATEWAY_RULES_OFF"], list);
    }
}

fn commit_source(fixture: &Fixture, source: &str) -> String {
    fs::write(fixture.0.join("checkout/src/unit.c"), source).unwrap();
    let output = Command::new("git")
        .current_dir(fixture.0.join("checkout"))
        .args([
            "-c",
            "user.name=Audit Fixture",
            "-c",
            "user.email=audit@example.invalid",
            "commit",
            "-am",
            "Changed upstream source",
            "--quiet",
            "--allow-empty",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new("git")
        .current_dir(fixture.0.join("checkout"))
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[test]
fn checkout_head_reanalyzes_changed_source_without_reusing_historical_evidence() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("int entry(void) { return 7; }");
    let changed = "extern void opaque(void); void entry(void) { opaque(); }";
    let revision = commit_source(&fixture, changed);
    // The historical default still rejects a different revision.
    assert!(!fixture.run().status.success());
    assert!(!fixture.0.join("output/report.json").exists());
    fixture.mutate("audit.json", |value| {
        value["source_revision_policy"] = json!("checkout_head")
    });
    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(fixture.0.join("output/report.json")).unwrap()).unwrap();
    assert_eq!(report["source_revision"], revision);
    assert_eq!(report["units"][0]["status"], "Unknown");
    assert!(report.get("source_contracts").is_none());
    let assumptions: Value = serde_json::from_slice(
        &fs::read(fixture.0.join("output/report.assumptions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        assumptions["source_revision_selection"]["resolved_commit"],
        revision
    );
    assert_eq!(
        assumptions["source_sha256"]["sample/src/unit.c"],
        sha256_hex(changed.as_bytes())
    );
    assert!(
        assumptions["source_revision_selection"]["source_specific_evidence"]
            .as_str()
            .unwrap()
            .starts_with("Unknown")
    );
}

#[test]
fn checkout_head_rejects_dirty_selected_bytes_and_historical_provider_evidence() {
    if !clang_available() {
        return;
    }
    for cause in ["dirty", "provider", "required", "unknown_policy"] {
        let fixture = Fixture::new("int entry(void) { return 7; }");
        fixture.mutate("audit.json", |value| {
            value["source_revision_policy"] = json!("checkout_head")
        });
        match cause {
            "dirty" => {
                // Even a selected file outside clean_paths must match the resolved commit.
                fixture.mutate("profiles/sample/profile.json", |value| {
                    value["pins"]["sample"]["clean_paths"] = json!(["other"])
                });
                fs::write(
                    fixture.0.join("checkout/src/unit.c"),
                    "int entry(void) { return 8; }",
                )
                .unwrap();
            }
            "provider" => fixture.mutate("audit.json", |value| {
                value["source_contracts"] = json!({
                    "configuration":"missing.json", "source_rules":"missing.qcore", "libraries":[]
                })
            }),
            "required" => fixture.mutate("profiles/sample/profile.json", |value| {
                value["libraries"]["sample"]["required_evidence"] =
                    json!(["native_allocation_ledger"])
            }),
            _ => fixture.mutate("audit.json", |value| {
                value["source_revision_policy"] = json!("anything")
            }),
        }
        let output = fixture.run();
        assert!(!output.status.success(), "{cause}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(match cause {
                "dirty" => "does not match its pinned bytes",
                "provider" => "supports resource projection only",
                "required" => "requires evidence",
                _ => "unknown variant",
            }),
            "{cause}: {error}"
        );
        assert!(!fixture.0.join("output/report.json").exists());
    }
}

#[test]
fn checkout_head_commit_change_retires_identity_even_with_identical_source() {
    if !clang_available() {
        return;
    }
    let source = "int entry(void) { return 7; }";
    let fixture = Fixture::new(source);
    fixture.mutate("audit.json", |value| {
        value["source_revision_policy"] = json!("checkout_head")
    });
    assert!(fixture.run().status.success());
    let before: Value = serde_json::from_slice(
        &fs::read(fixture.0.join("output/report.assumptions.json")).unwrap(),
    )
    .unwrap();
    let revision = commit_source(&fixture, source);
    assert!(fixture.run().status.success());
    let after: Value = serde_json::from_slice(
        &fs::read(fixture.0.join("output/report.assumptions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        after["source_revision_selection"]["resolved_commit"],
        revision
    );
    assert_eq!(before["source_sha256"], after["source_sha256"]);
    assert_ne!(
        before["compile_tasks"][0]["identity_sha256"],
        after["compile_tasks"][0]["identity_sha256"]
    );
}

fn inventory(fixture: &Fixture, arguments: &[&str]) -> Value {
    use qcore_audit::compile_db::CompileEntry;
    use qcore_audit::current::{CurrentAudit, distinct_configurations};
    let checkout = fixture.0.join("checkout");
    let profile =
        qcore_contract::profile_loader::load_profile_directory(&fixture.0.join("profiles/sample"))
            .unwrap();
    let mut audit =
        CurrentAudit::new(&checkout, &profile, &fixture.0.join("contracts.json")).unwrap();
    let entries = distinct_configurations(arguments.iter().map(|argument| CompileEntry {
        directory: checkout.clone(),
        file: checkout.join("src/unit.c"),
        command: format!("cc -std=c11 -c src/unit.c {argument}"),
    }));
    for entry in entries {
        audit
            .audit_entry("sample", &entry.file, &entry, None)
            .unwrap();
    }
    audit
        .finish(Some(&fixture.0.join("inventory.json")))
        .unwrap();
    serde_json::from_slice(&fs::read(fixture.0.join("inventory.json")).unwrap()).unwrap()
}

#[test]
fn current_inventory_keeps_distinct_commands_and_unsupported_configurations_unknown() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new(
        "extern void opaque(void);\nvoid entry(void) {\n#ifdef CALL_OPAQUE\nopaque();\n#endif\n}\n",
    );
    let report = inventory(&fixture, &["", "-DCALL_OPAQUE", "-unsupported", ""]);
    let units = report["units"].as_array().unwrap();
    assert_eq!(units.len(), 3);
    assert_eq!(
        units.iter().filter(|unit| unit["status"] == "Safe").count(),
        1
    );
    assert_eq!(
        units
            .iter()
            .filter(|unit| unit["status"] == "Unknown")
            .count(),
        2
    );
    let rejected = units
        .iter()
        .find(|unit| {
            unit["compile_configuration"]["command"]
                .as_str()
                .unwrap()
                .contains("-unsupported")
        })
        .unwrap();
    assert!(rejected["reason"].as_str().unwrap().contains("unsupported"));
    assert!(
        rejected["compile_configuration"]
            .get("compile_task_identity")
            .is_none()
    );
    assert_eq!(
        units
            .iter()
            .map(|unit| unit["compile_configuration"]["inventory_sha256"]
                .as_str()
                .unwrap())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
}

#[test]
fn current_inventory_reanalyzes_new_commits_and_never_admits_dirty_source() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("int entry(void) { return 1; }");
    let first = inventory(&fixture, &[""]);
    let revision = commit_source(&fixture, "int entry(void) { return 1; }");
    let second = inventory(&fixture, &[""]);
    assert_eq!(second["fluent_bit_revision"], revision);
    assert_ne!(
        first["units"][0]["compile_configuration"]["inventory_sha256"],
        second["units"][0]["compile_configuration"]["inventory_sha256"]
    );
    commit_source(
        &fixture,
        "extern void opaque(void); void entry(void) { opaque(); }",
    );
    assert_eq!(inventory(&fixture, &[""])["units"][0]["status"], "Unknown");
    fs::write(
        fixture.0.join("checkout/src/unit.c"),
        "int entry(void) { return 1; }",
    )
    .unwrap();
    let dirty = inventory(&fixture, &[""]);
    assert_eq!(dirty["units"][0]["status"], "Unknown");
    assert!(
        dirty["units"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("does not match its pinned bytes")
    );
}

#[test]
fn a_second_codebase_runs_from_only_profile_contract_and_compile_database_files() {
    if !clang_available() {
        return;
    }
    for (source, expected) in [
        ("int value(void) { return 7; }", "Safe"),
        (
            "extern void *alloc(void); void use(void) { void *p = alloc(); }",
            "Unsafe",
        ),
        (
            "extern void opaque(void); void use(void) { opaque(); }",
            "Unknown",
        ),
    ] {
        let fixture = Fixture::new(source);
        let output = fixture.run();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value =
            serde_json::from_slice(&fs::read(fixture.0.join("output/report.json")).unwrap())
                .unwrap();
        assert_eq!(report["units"][0]["status"], expected);
        let assumptions: Value = serde_json::from_slice(
            &fs::read(fixture.0.join("output/report.assumptions.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            assumptions["source_sha256"]["sample/src/unit.c"],
            sha256_hex(source.as_bytes())
        );
        assert!(
            assumptions["profile"]["profile_digest"]
                .as_str()
                .unwrap()
                .len()
                == 64
        );
    }
}

#[cfg(unix)]
#[test]
fn compilation_database_path_aliases_select_the_canonical_checkout_source() {
    use std::os::unix::fs::symlink;

    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("int value(void) { return 7; }");
    let alias = fixture.0.join("checkout-alias");
    symlink(fixture.0.join("checkout"), &alias).unwrap();
    fixture.mutate("compile_commands.json", |database| {
        database[0]["directory"] = json!(alias);
    });

    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(fixture.0.join("output/report.json")).unwrap()).unwrap();
    assert_eq!(report["units"][0]["status"], "Safe");
}

/// Before the compile task carried an identity, the recorded assumptions bound
/// only the selected `.c` bytes and the database. A used header could flip a
/// unit's verdict while every recorded input stayed byte-identical, so evidence
/// filed under those inputs could be reused for a different analysis.
#[test]
fn a_used_header_change_invalidates_the_recorded_compile_task_identity() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new(
        "#include \"effect.h\"\nextern void opaque(void); void entry(void) { EFFECT; }",
    );
    let header = fixture.0.join("checkout/src/effect.h");
    let mut observed = Vec::new();
    for (effect, expected) in [("((void)0)", "Safe"), ("opaque()", "Unknown")] {
        fs::write(&header, format!("#define EFFECT {effect}\n")).unwrap();
        let output = fixture.run();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value =
            serde_json::from_slice(&fs::read(fixture.0.join("output/report.json")).unwrap())
                .unwrap();
        assert_eq!(report["units"][0]["status"], expected);
        let assumptions: Value = serde_json::from_slice(
            &fs::read(fixture.0.join("output/report.assumptions.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(assumptions["compile_task_identity_version"], 1);
        let task = assumptions["compile_tasks"][0].clone();
        assert_eq!(task["file"], "src/unit.c");
        assert_eq!(task["library"], "sample");
        observed.push((
            assumptions["source_sha256"].clone(),
            assumptions["compile_database_sha256"].clone(),
            task["identity"]["raw_source_sha256"].clone(),
            task["identity_sha256"].clone(),
        ));
    }
    let [safe, unknown] = observed.as_slice() else {
        unreachable!()
    };
    assert_eq!(
        safe.0, unknown.0,
        "the selected source bytes did not change"
    );
    assert_eq!(safe.1, unknown.1, "the compilation database did not change");
    assert_eq!(safe.2, unknown.2);
    assert_ne!(
        safe.3, unknown.3,
        "a used header changed the analyzed code without changing the task identity"
    );
}

/// Real CMake databases emit several commands per source file. Ambiguity must
/// not resolve itself; identical repeats describe one task and are audited once.
#[test]
fn repeated_and_conflicting_compile_records_for_one_file_are_distinguished() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("int value(void) { return 7; }");
    fixture.mutate("compile_commands.json", |database| {
        let repeat = database[0].clone();
        database.as_array_mut().unwrap().push(repeat);
    });
    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(fixture.0.join("output/report.json")).unwrap()).unwrap();
    assert_eq!(report["units"].as_array().unwrap().len(), 1);
    let assumptions: Value = serde_json::from_slice(
        &fs::read(fixture.0.join("output/report.assumptions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(assumptions["compile_tasks"].as_array().unwrap().len(), 1);

    // Options the gateway transports in order are enough to make two records
    // conflict, even when both would preprocess identically.
    for arguments in [
        json!(["cc", "-std=c11", "-DA=1", "-DB=1", "-c", "src/unit.c"]),
        json!(["cc", "-std=c11", "-c", "src/unit.c", "-o", "other.o"]),
    ] {
        let fixture = Fixture::new("int value(void) { return 7; }");
        fixture.mutate("compile_commands.json", |database| {
            let mut extra = database[0].clone();
            extra["arguments"] = arguments.clone();
            database.as_array_mut().unwrap().push(extra);
        });
        let output = fixture.run();
        assert!(
            !output.status.success(),
            "{arguments} unexpectedly accepted"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("conflicting compilation commands"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!fixture.0.join("output/report.json").exists());
    }
}

#[test]
fn absent_selections_contracts_and_unsupported_flags_fail_closed() {
    for failure in ["selection", "contract", "flags", "conflict", "evidence"] {
        let fixture = Fixture::new("int value(void) { return 7; }");
        match failure {
            "selection" => fixture.mutate("audit.json", |config| {
                config["libraries"] = json!(["absent"])
            }),
            "evidence" => fixture.mutate("profiles/sample/profile.json", |profile| {
                profile["libraries"]["sample"]["required_evidence"] =
                    json!(["native_allocation_ledger"]);
            }),
            "contract" => {
                fs::remove_file(fixture.0.join("alloc.qcore")).unwrap();
            }
            "flags" => fixture.mutate("compile_commands.json", |database| {
                database[0]["arguments"]
                    .as_array_mut()
                    .unwrap()
                    .push("-include=hidden.h".into())
            }),
            "conflict" => fixture.mutate("compile_commands.json", |database| {
                let mut extra = database[0].clone();
                extra["arguments"][1] = "-std=gnu11".into();
                database.as_array_mut().unwrap().push(extra);
            }),
            _ => unreachable!(),
        }
        let output = fixture.run();
        assert!(!output.status.success(), "{failure} unexpectedly accepted");
        assert!(!fixture.0.join("output/report.json").exists());
    }
}

#[test]
fn selected_source_bytes_must_match_the_pin_even_outside_declared_clean_paths() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("int value(void) { return 7; }");
    fixture.mutate("profiles/sample/profile.json", |profile| {
        profile["pins"]["sample"]["clean_paths"] = json!(["irrelevant"]);
        profile["libraries"]["sample"]["audited_files"] = json!([]);
    });
    fs::write(
        fixture.0.join("checkout/src/unit.c"),
        "int value(void) { return 8; }",
    )
    .unwrap();
    let output = fixture.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("does not match its pinned bytes"));
}

#[test]
fn header_search_preserves_unresolved_call_instead_of_publishing_safe() {
    if !clang_available() {
        return;
    }
    for local_shadow in [false, true] {
        let fixture = Fixture::new(
            "#include <effect.h>\nextern void opaque(void); void entry(void) { EFFECT; }",
        );
        fs::create_dir_all(fixture.0.join("checkout/regular")).unwrap();
        fs::create_dir_all(fixture.0.join("checkout/system")).unwrap();
        fs::write(
            fixture.0.join("checkout/regular/effect.h"),
            "#define EFFECT opaque()\n",
        )
        .unwrap();
        let shadow = if local_shadow { "src" } else { "system" };
        fs::write(
            fixture.0.join(format!("checkout/{shadow}/effect.h")),
            "#define EFFECT ((void)0)\n",
        )
        .unwrap();
        fixture.mutate("compile_commands.json", |database| {
            database[0]["arguments"] = json!([
                "clang",
                "-std=c11",
                "-isystem",
                "system",
                "-Iregular",
                "-c",
                "src/unit.c"
            ]);
        });
        let output = fixture.run();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value =
            serde_json::from_slice(&fs::read(fixture.0.join("output/report.json")).unwrap())
                .unwrap();
        assert_eq!(
            report["units"][0]["status"], "Unknown",
            "local shadow: {local_shadow}"
        );
    }
}

#[cfg(not(windows))]
#[test]
fn pic_build_flag_preserves_the_unresolved_call_selected_by_preprocessing() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new(
        "#ifdef __PIE__\n#define EFFECT ((void)0)\n#else\n#define EFFECT opaque()\n#endif\nextern void opaque(void); void entry(void) { EFFECT; }",
    );
    fixture.mutate("compile_commands.json", |database| {
        database[0]["arguments"] = json!([
            "clang",
            "-std=c11",
            "-Wall",
            "-g",
            "-fPIC",
            "-Wa,--noexecstack",
            "-c",
            "src/unit.c"
        ]);
    });
    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(fixture.0.join("output/report.json")).unwrap()).unwrap();
    assert_eq!(report["units"][0]["status"], "Unknown");
}

#[test]
fn quoted_includes_resolve_against_the_source_not_the_audit_working_directory() {
    // A quoted include is resolved against the *including file's* directory.
    // Feeding the text through standard input instead makes the gateway's own
    // working directory play that role, so an unrelated header sitting next to
    // wherever the audit was started could silently replace the real one and
    // publish a verdict for a translation unit nobody compiled.
    let fixture = Fixture::new(
        "#include \"effect.h\"\nextern void opaque(void); void entry(void) { EFFECT; }",
    );
    fs::write(
        fixture.0.join("checkout/src/effect.h"),
        "#define EFFECT opaque()\n",
    )
    .unwrap();
    let shadow = fixture.0.join("shadow");
    fs::create_dir_all(&shadow).unwrap();
    fs::write(shadow.join("effect.h"), "#define EFFECT ((void)0)\n").unwrap();
    fixture.mutate("compile_commands.json", |database| {
        database[0]["arguments"] = json!(["clang", "-std=c11", "-c", "src/unit.c"]);
    });
    // The shadowing directory is the one that used to win. Both runs must now
    // report the unresolved call the recorded compile command actually sees.
    for directory in [shadow.clone(), fixture.0.clone()] {
        let output = fixture.run_in(&directory);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value =
            serde_json::from_slice(&fs::read(fixture.0.join("output/report.json")).unwrap())
                .unwrap();
        assert_eq!(
            report["units"][0]["status"],
            "Unknown",
            "started in {}",
            directory.display()
        );
    }
}

#[test]
fn source_file_name_macros_answer_with_the_recorded_input() {
    // `__FILE__` under the standard-input transport expanded to `<stdin>`,
    // which is not any file the recorded build compiles. The configured audit
    // now hands Clang the recorded input word. This is the end-to-end smoke:
    // the byte-exact comparison against a direct compile is in the
    // `filename_sensitive_preprocessing_matches_the_recorded_compile_command`
    // differential, which reads the preprocessed text itself.
    let fixture =
        Fixture::new("const char *where(void) { return __FILE__; }\nint value(void) { return 7; }");
    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(fixture.0.join("output/report.json")).unwrap()).unwrap();
    assert_eq!(report["units"][0]["status"], "Safe");
}

#[test]
fn deferred_source_location_builtins_reject_instead_of_answering_for_the_wrong_file() {
    // These are answered after preprocessing, where the gateway always reads
    // standard input. No transport here makes them true, so the unit fails
    // closed rather than being projected under a guessed file name.
    let fixture = Fixture::new("const char *where(void) { return __builtin_FILE(); }");
    let output = fixture.run();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stderr.contains("__builtin_FILE") || stdout.contains("__builtin_FILE"),
        "{stderr}{stdout}"
    );
}

// AUDIT-IR-FRAGMENTS: capture is a side artifact, not a new verdict path.
fn run_ir_capture(fixture: &Fixture, directory: Option<&std::path::Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
    command
        .current_dir(&fixture.0)
        .arg(fixture.0.join("audit.json"));
    match directory {
        Some(path) => {
            command.env("QCORE_AUDIT_IR_DIR", path);
        }
        None => {
            command.env_remove("QCORE_AUDIT_IR_DIR");
        }
    }
    command.output().unwrap()
}

#[test]
fn ir_capture_round_trips_real_cfgs_without_changing_configured_report() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new(
        "void external(void); void subject(void) { return; } void undecided(void) { external(); }\n",
    );
    let baseline = run_ir_capture(&fixture, None);
    assert!(
        baseline.status.success(),
        "{}",
        String::from_utf8_lossy(&baseline.stderr)
    );
    let report = fs::read(fixture.0.join("output/report.json")).unwrap();
    let artifacts = fixture.0.join("ir");
    let captured = run_ir_capture(&fixture, Some(&artifacts));
    assert!(
        captured.status.success(),
        "{}",
        String::from_utf8_lossy(&captured.stderr)
    );
    assert_eq!(
        fs::read(fixture.0.join("output/report.json")).unwrap(),
        report
    );
    // Walks below inspect only this test's own temporary output directory.
    let unit = fs::read_dir(artifacts.join("units"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let index_path = fs::read_dir(unit).unwrap().next().unwrap().unwrap().path();
    assert_eq!(index_path.extension().unwrap(), "qc");
    for object in fs::read_dir(artifacts.join("objects")).unwrap() {
        assert_eq!(object.unwrap().path().extension().unwrap(), "qc");
    }
    let index: qcore_c_gateway::audit_ir_capture::CaptureIndex =
        serde_json::from_slice(&fs::read(index_path).unwrap()).unwrap();
    assert_eq!(
        index
            .targets
            .iter()
            .map(|t| t.function.as_str())
            .collect::<Vec<_>>(),
        ["subject", "undecided"]
    );
    assert!(index.requires_fresh_lowering);
    assert!(index.targets.iter().all(|t| !t.cfgs.is_empty()));
    let original_count = fs::read_dir(artifacts.join("objects")).unwrap().count();
    let again = run_ir_capture(&fixture, Some(&artifacts));
    assert!(
        again.status.success(),
        "{}",
        String::from_utf8_lossy(&again.stderr)
    );
    assert_eq!(
        fs::read_dir(artifacts.join("objects")).unwrap().count(),
        original_count
    );
    assert_eq!(
        fs::read(fixture.0.join("output/report.json")).unwrap(),
        report
    );
}

#[test]
fn corrupt_ir_capture_aborts_without_overwriting_or_publishing_a_new_report() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new("void subject(void) { return; }\n");
    let artifacts = fixture.0.join("ir");
    let captured = run_ir_capture(&fixture, Some(&artifacts));
    assert!(
        captured.status.success(),
        "{}",
        String::from_utf8_lossy(&captured.stderr)
    );
    let unit = fs::read_dir(artifacts.join("units"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .file_name();
    let corrupt = artifacts
        .join("objects")
        .join(format!("{}.qc", unit.to_string_lossy()));
    fs::write(&corrupt, b"truncated").unwrap();
    fs::remove_file(fixture.0.join("output/report.json")).unwrap();
    let output = run_ir_capture(&fixture, Some(&artifacts));
    assert!(!output.status.success());
    assert!(!fixture.0.join("output/report.json").exists());
    assert_eq!(fs::read(corrupt).unwrap(), b"truncated");
    let disabled = run_ir_capture(&fixture, None);
    assert!(
        disabled.status.success(),
        "{}",
        String::from_utf8_lossy(&disabled.stderr)
    );
}
