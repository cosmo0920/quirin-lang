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
        self.run_in(&self.0.clone())
    }
    /// Run the audit from an explicit working directory. A unit's meaning must
    /// not depend on where the audit happens to be started.
    fn run_in(&self, directory: &std::path::Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_qcore-audit"))
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
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
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
