//! Card W45-AUDIT-CHECKPOINTS: `quirin-audit current --checkpoint-dir` end to
//! end on a two-library fixture — one complete `.qo` per compile configuration,
//! a report byte-identical to the whole run's at every job count, resume, streaming
//! composition with its refusals, and the YAML/TOML decomposition round trip.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use qcore_audit::checkpoint::{Checkpoint, CheckpointReader, TextFormat, decompose, parse_text};
use qcore_contract::profile_loader::sha256_hex;
use serde_json::json;

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
            eprintln!("skipping Clang-dependent checkpoint test: {reason}");
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

/// Two libraries' units; some leak on one path, one is data only, so the
/// groups have distinct verdicts and diagnostics to fold.
fn units() -> Vec<(String, String)> {
    let mut units = Vec::new();
    for (library, count) in [("alpha", 3), ("beta", 2)] {
        for index in 0..count {
            units.push((
                format!("{library}/{library}_{index}.c"),
                format!(
                    "unsigned char *alloc(void);\nvoid dispose(unsigned char *p);\n\
                     void {library}_{index}(int c) {{ unsigned char *p = alloc(); if (c > {index}) dispose(p); }}\n\
                     void {library}_{index}_ok(void) {{ unsigned char *p = alloc(); dispose(p); }}\n"
                ),
            ));
        }
    }
    units.push(("beta/data.c".to_owned(), "int beta_table[4];\n".to_owned()));
    units
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

struct Fixture(PathBuf);

impl Fixture {
    fn new(driver: &Path) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-checkpoint-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let checkout = root.join("checkout");
        for (path, text) in units() {
            let path = checkout.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        fs::write(checkout.join("alpha/unrecorded.c"), "int unrecorded;\n").unwrap();
        git(&checkout, &["init", "--quiet"]);
        git(&checkout, &["add", "alpha", "beta"]);
        git(
            &checkout,
            &[
                "-c",
                "user.name=Checkpoint Fixture",
                "-c",
                "user.email=checkpoint@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Checkpoint fixture",
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
        let library = |name: &str| {
            json!({
                "pin":"sample", "source_roots":[{"root": name, "selection":"top_level"}],
                "audited_files": units().iter().filter(|(path, _)| path.starts_with(name)).map(|(path, text)| json!({"path": path, "sha256": sha256_hex(text.as_bytes())})).collect::<Vec<_>>(),
                "source_witnesses":[], "permitted_mechanisms":[], "operation_summaries":{},
                "api_dispositions":{}, "accepted_builds":[], "required_evidence":[], "typed_import":null,
                "callback_summaries":{}, "boolean_field_models":[], "provider":null,
            })
        };
        let profile = root.join("profiles/sample");
        fs::create_dir_all(&profile).unwrap();
        fs::write(profile.join("profile.json"), json!({
            "schema_version":1, "source_set":"sample",
            "pins":{"sample":{"git_commit":commit,"clean_paths":[],"description":"checkpoint fixture"}},
            "libraries":{"alpha": library("alpha"), "beta": library("beta")},
            "resource_contract_manifest": "../../contracts.json",
            "provider_manifests":[],
        }).to_string()).unwrap();
        fs::write(
            profile.join("audit.json"),
            json!({"schema_version":1,"libraries":[
                {"library":"alpha","label":null},
                {"library":"beta","label":"beta-group"},
            ]})
            .to_string(),
        )
        .unwrap();
        fs::write(
            root.join("compile_commands.json"),
            json!(
                units()
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
        fs::write(
            root.join("selection.json"),
            json!({"schema_version": 1, "name": "picked", "groups": [
                {"library": "alpha", "files": ["alpha/alpha_0.c", "alpha/unrecorded.c"]},
                {"library": "beta", "label": "beta-picked", "files": ["beta/beta_1.c"]},
            ]})
            .to_string(),
        )
        .unwrap();
        Self(root)
    }

    fn run(&self, out: &str, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
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

    fn checkpoint(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(&self.0)
            .arg("checkpoint")
            .args(args)
            .output()
            .unwrap()
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }

    fn outputs(&self, out: &str) -> Vec<Vec<u8>> {
        [
            "report.json",
            "report.contracts.json",
            "report.chunk-contracts.json",
            "report.source-effects.json",
            "monkey-source.json",
        ]
        .iter()
        .map(|name| fs::read(self.0.join(out).join(name)).unwrap())
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

/// The console without the measured `preparation:` line.
fn console(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.starts_with("preparation:"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn content(path: &Path) -> String {
    CheckpointReader::open(path)
        .unwrap()
        .content_sha256()
        .unwrap()
}

fn checkpoint_paths(fixture: &Fixture, directory: &str) -> Vec<PathBuf> {
    let record: serde_json::Value = serde_json::from_slice(
        &fs::read(fixture.path(&format!("{directory}/checkpoint-run.json"))).unwrap(),
    )
    .unwrap();
    record["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|group| {
            fixture.path(&format!(
                "{directory}/{}.qo",
                group["group"].as_str().unwrap()
            ))
        })
        .collect()
}

fn checkpoint_for(fixture: &Fixture, directory: &str, file: &str) -> PathBuf {
    checkpoint_paths(fixture, directory)
        .into_iter()
        .find(|path| {
            let checkpoint = Checkpoint::from_bytes(&fs::read(path).unwrap()).unwrap();
            checkpoint.identity.group.units[0].file == file
        })
        .unwrap()
}

#[test]
fn per_configuration_checkpoints_match_whole_and_resume_independently() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let whole = fixture.run("whole", &["--jobs", "1"]);
    succeeded(&whole);
    let one = fixture.run("one", &["--jobs", "1", "--checkpoint-dir", "qo-1"]);
    succeeded(&one);
    let four = fixture.run("four", &["--jobs", "4", "--checkpoint-dir", "qo-4"]);
    succeeded(&four);
    assert_eq!(fixture.outputs("one"), fixture.outputs("whole"));
    assert_eq!(fixture.outputs("four"), fixture.outputs("whole"));
    assert_eq!(console(&one), console(&whole));
    assert_eq!(console(&four), console(&whole));
    let paths = checkpoint_paths(&fixture, "qo-1");
    assert_eq!(paths.len(), 6);
    for (left, right) in paths.iter().zip(checkpoint_paths(&fixture, "qo-4")) {
        let checkpoint = Checkpoint::from_bytes(&fs::read(left).unwrap()).unwrap();
        assert_eq!(checkpoint.identity.group.units.len(), 1);
        assert_eq!(checkpoint.preparation.units.len(), 1);
        assert_eq!(checkpoint.lowering.units.len(), 1);
        assert_eq!(checkpoint.verdicts.units.len(), 1);
        assert_eq!(checkpoint.certificates.units.len(), 1);
        assert_eq!(content(left), content(&right));
        succeeded(&fixture.checkpoint(&["verify", left.to_str().unwrap()]));
    }
    let alpha = checkpoint_for(&fixture, "qo-1", "alpha/alpha_0.c");
    let unchanged = fs::read(&alpha).unwrap();
    let again = fixture.run(
        "again",
        &["--jobs", "2", "--checkpoint-dir", "qo-1", "--resume"],
    );
    succeeded(&again);
    assert_eq!(fixture.outputs("again"), fixture.outputs("whole"));
    assert_eq!(fs::read(&alpha).unwrap(), unchanged);
    assert_eq!(
        String::from_utf8_lossy(&again.stderr)
            .matches("reused (identity equal)")
            .count(),
        6
    );
    fs::write(
        fixture.path("checkout/beta/beta_1.c"),
        "void beta_1(void) {}\n",
    )
    .unwrap();
    let changed = fixture.run(
        "changed",
        &["--jobs", "2", "--checkpoint-dir", "qo-1", "--resume"],
    );
    succeeded(&changed);
    let stderr = String::from_utf8_lossy(&changed.stderr);
    assert_eq!(
        stderr.matches("reused (identity equal)").count(),
        5,
        "{stderr}"
    );
    assert!(stderr.contains("compile configuration differs"), "{stderr}");
    succeeded(&fixture.run("changed-whole", &["--jobs", "2"]));
    assert_eq!(fixture.outputs("changed"), fixture.outputs("changed-whole"));
    let mut tampered = fs::read(&alpha).unwrap();
    *tampered.last_mut().unwrap() ^= 1;
    fs::write(&alpha, tampered).unwrap();
    let repaired = fixture.run(
        "repaired",
        &["--jobs", "2", "--checkpoint-dir", "qo-1", "--resume"],
    );
    succeeded(&repaired);
    assert!(String::from_utf8_lossy(&repaired.stderr).contains("checkpoint refused:"));
    assert_eq!(
        fixture.outputs("repaired"),
        fixture.outputs("changed-whole")
    );
}

#[test]
fn compose_requires_every_distinct_checkpoint_and_round_trips_text() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    succeeded(&fixture.run("whole", &["--jobs", "2"]));
    succeeded(&fixture.run("ck", &["--jobs", "2", "--checkpoint-dir", "qo"]));
    let paths = checkpoint_paths(&fixture, "qo");
    let mut args = vec!["compose"];
    for path in paths.iter().rev() {
        args.push(path.to_str().unwrap());
    }
    args.extend([
        "--report",
        "composed/report.json",
        "--monkey-source-report",
        "composed/monkey-source.json",
    ]);
    let composed = fixture.checkpoint(&args);
    succeeded(&composed);
    assert_eq!(fixture.outputs("composed"), fixture.outputs("whole"));
    let whole = fixture.run("whole-console", &["--jobs", "2"]);
    succeeded(&whole);
    assert_eq!(console(&composed), console(&whole));
    let mut missing = vec!["compose"];
    missing.extend(paths[..paths.len() - 1].iter().map(|p| p.to_str().unwrap()));
    refused(&fixture.checkpoint(&missing), "missing:");
    let mut duplicate = vec!["compose", paths[0].to_str().unwrap()];
    duplicate.extend(paths.iter().map(|p| p.to_str().unwrap()));
    refused(&fixture.checkpoint(&duplicate), "given twice");
    fs::write(fixture.path("contracts/heap.qcore"), format!("{HEAP}\n")).unwrap();
    succeeded(&fixture.run("other", &["--jobs", "2", "--checkpoint-dir", "qo-other"]));
    let mut mixed = paths
        .iter()
        .map(|p| p.to_str().unwrap())
        .collect::<Vec<_>>();
    let other = checkpoint_paths(&fixture, "qo-other");
    mixed[1] = other[1].to_str().unwrap();
    mixed.insert(0, "compose");
    refused(&fixture.checkpoint(&mixed), "disagree on the contract set");
    let bytes = fs::read(&paths[0]).unwrap();
    fs::write(fixture.path("truncated.qo"), &bytes[..bytes.len() - 5]).unwrap();
    refused(
        &fixture.checkpoint(&["verify", "truncated.qo"]),
        "truncated",
    );
    let mut corrupt = paths
        .iter()
        .map(|p| p.to_str().unwrap())
        .collect::<Vec<_>>();
    corrupt[0] = "truncated.qo";
    corrupt.insert(0, "compose");
    refused(&fixture.checkpoint(&corrupt), "truncated");
    for format in [TextFormat::Yaml, TextFormat::Toml] {
        let checkpoint = Checkpoint::from_bytes(&bytes).unwrap();
        let text = decompose(&checkpoint, format).unwrap();
        assert_eq!(
            parse_text(&text, format).unwrap().to_bytes().unwrap(),
            bytes
        );
    }
}

#[test]
fn multiple_configurations_of_one_source_get_distinct_complete_files() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let db = fixture.path("compile_commands.json");
    let mut entries: serde_json::Value = serde_json::from_slice(&fs::read(&db).unwrap()).unwrap();
    let mut second = entries[0].clone();
    second["arguments"]
        .as_array_mut()
        .unwrap()
        .insert(2, json!("-DSECOND_CONFIGURATION=1"));
    entries.as_array_mut().unwrap().insert(1, second);
    fs::write(&db, serde_json::to_vec(&entries).unwrap()).unwrap();
    succeeded(&fixture.run("whole", &["--jobs", "2"]));
    succeeded(&fixture.run("ck", &["--jobs", "2", "--checkpoint-dir", "qo"]));
    assert_eq!(fixture.outputs("ck"), fixture.outputs("whole"));
    let paths = checkpoint_paths(&fixture, "qo");
    assert_eq!(paths.len(), 7);
    let same_source = paths
        .iter()
        .filter(|path| {
            Checkpoint::from_bytes(&fs::read(path).unwrap())
                .unwrap()
                .identity
                .group
                .units[0]
                .file
                == "alpha/alpha_0.c"
        })
        .collect::<Vec<_>>();
    assert_eq!(same_source.len(), 2);
    assert_ne!(same_source[0], same_source[1]);
    assert_ne!(content(same_source[0]), content(same_source[1]));
    let resumed = fixture.run(
        "resumed",
        &["--jobs", "2", "--checkpoint-dir", "qo", "--resume"],
    );
    succeeded(&resumed);
    assert_eq!(
        String::from_utf8_lossy(&resumed.stderr)
            .matches("reused (identity equal)")
            .count(),
        7
    );
    assert_eq!(fixture.outputs("resumed"), fixture.outputs("whole"));
}

#[test]
fn a_re_pin_invalidates_every_unit_checkpoint() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    succeeded(&fixture.run("before", &["--jobs", "2", "--checkpoint-dir", "qo"]));
    let alpha = checkpoint_for(&fixture, "qo", "alpha/alpha_0.c");
    let before = Checkpoint::from_bytes(&fs::read(&alpha).unwrap()).unwrap();
    assert!(before.identity.run.contracts.locks.is_empty());
    let pinned = Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
        .current_dir(&fixture.0)
        .args(["pin", "--source-root", "checkout", "--modules", "contracts"])
        .output()
        .unwrap();
    succeeded(&pinned);
    let after = fixture.run(
        "after",
        &["--jobs", "2", "--checkpoint-dir", "qo", "--resume"],
    );
    succeeded(&after);
    let stderr = String::from_utf8_lossy(&after.stderr);
    assert_eq!(
        stderr.matches("recomputed (contract set differs)").count(),
        6,
        "{stderr}"
    );
    let now = Checkpoint::from_bytes(&fs::read(&alpha).unwrap()).unwrap();
    assert_eq!(now.identity.run.contracts.locks.len(), 1);
    assert_ne!(
        now.identity.run.contracts.pin_evaluation_sha256,
        before.identity.run.contracts.pin_evaluation_sha256
    );
    succeeded(&fixture.run("after-whole", &["--jobs", "2"]));
    assert_eq!(fixture.outputs("after"), fixture.outputs("after-whole"));
    assert_ne!(fixture.outputs("after"), fixture.outputs("before"));
    refused(
        &fixture.run("bad", &["--resume"]),
        "--resume needs --checkpoint-dir",
    );
}

#[test]
fn selection_with_unrecorded_file_resumes() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let selection = ["--selection", "selection.json", "--jobs", "2"];
    succeeded(&fixture.run("whole", &selection));
    let mut args = selection.to_vec();
    args.extend(["--checkpoint-dir", "qo"]);
    succeeded(&fixture.run("first", &args));
    assert_eq!(checkpoint_paths(&fixture, "qo").len(), 3);
    args.push("--resume");
    let again = fixture.run("again", &args);
    succeeded(&again);
    assert_eq!(
        String::from_utf8_lossy(&again.stderr)
            .matches("reused (identity equal)")
            .count(),
        3
    );
    assert_eq!(fixture.outputs("again"), fixture.outputs("whole"));
    let selection_path = fixture.path("selection.json");
    let mut selection_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&selection_path).unwrap()).unwrap();
    selection_json["groups"][0]["files"]
        .as_array_mut()
        .unwrap()
        .reverse();
    fs::write(
        &selection_path,
        serde_json::to_vec(&selection_json).unwrap(),
    )
    .unwrap();
    let reordered = fixture.run("reordered", &args);
    succeeded(&reordered);
    assert_eq!(
        String::from_utf8_lossy(&reordered.stderr)
            .matches("reused (identity equal)")
            .count(),
        3
    );
    let reordered_whole = fixture.run("reordered-whole", &selection);
    succeeded(&reordered_whole);
    assert_eq!(
        fixture.outputs("reordered"),
        fixture.outputs("reordered-whole")
    );
    assert_eq!(console(&reordered), console(&reordered_whole));
}

#[test]
fn interrupted_run_preserves_completed_units_for_resume() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let mut child = Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
        .current_dir(&fixture.0)
        .args([
            "current",
            "--checkout",
            "checkout",
            "--profile-directory",
            "profiles/sample",
            "--compile-database",
            "compile_commands.json",
            "--report",
            "interrupted/report.json",
            "--checkpoint-dir",
            "qo",
            "--jobs",
            "1",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let completed = loop {
        let count = fs::read_dir(fixture.path("qo"))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "qo"))
                    .count()
            })
            .unwrap_or(0);
        if count > 0 {
            break count;
        }
        assert!(
            Instant::now() < deadline,
            "no unit checkpoint before timeout"
        );
        assert!(
            child.try_wait().unwrap().is_none(),
            "run exited before first checkpoint"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    assert!(completed < 6, "run finished before interruption point");
    child.kill().unwrap();
    let stopped = child.wait().unwrap();
    assert!(!stopped.success());
    let resumed = fixture.run(
        "resumed",
        &["--jobs", "2", "--checkpoint-dir", "qo", "--resume"],
    );
    succeeded(&resumed);
    let stderr = String::from_utf8_lossy(&resumed.stderr);
    assert!(stderr.contains("reused (identity equal)"), "{stderr}");
    assert!(stderr.contains("recomputed (no checkpoint)"), "{stderr}");
    assert_eq!(checkpoint_paths(&fixture, "qo").len(), 6);
    succeeded(&fixture.run("whole", &["--jobs", "2"]));
    assert_eq!(fixture.outputs("resumed"), fixture.outputs("whole"));
}
