use qcore_contract::profile_loader::sha256_hex;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
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
            include_str!("fixtures/alloc_acquire.qcore"),
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
        Command::new(env!("CARGO_BIN_EXE_qcore-audit"))
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
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
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

#[test]
fn a_second_codebase_runs_from_only_profile_contract_and_compile_database_files() {
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

/// Before the compile task carried an identity, the recorded assumptions bound
/// only the selected `.c` bytes and the database. A used header could flip a
/// unit's verdict while every recorded input stayed byte-identical, so evidence
/// filed under those inputs could be reused for a different analysis.
#[test]
fn a_used_header_change_invalidates_the_recorded_compile_task_identity() {
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
