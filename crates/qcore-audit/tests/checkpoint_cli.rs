//! Card W45-AUDIT-CHECKPOINTS: `quirin-audit current --checkpoint-dir` end to
//! end on a two-group fixture — one `.qo` per group, a report byte-identical
//! to the whole run's at every job count, identity-guarded resume, streaming
//! composition with its refusals, and the YAML/TOML decomposition round trip.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

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
        fs::write(root.join("heap.qcore"), HEAP).unwrap();
        fs::write(
            root.join("contracts.json"),
            json!({"schema_version": 1, "modules": ["heap.qcore"]}).to_string(),
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

#[test]
fn a_checkpointed_run_writes_one_checkpoint_per_group_and_the_whole_runs_outputs() {
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
    for group in ["alpha", "beta-group"] {
        let at_one = fixture.path(&format!("qo-1/{group}.qo"));
        let at_four = fixture.path(&format!("qo-4/{group}.qo"));
        // Every section but provenance is the same at every job count.
        assert_eq!(content(&at_one), content(&at_four), "{group}");
        let verified = fixture.checkpoint(&["verify", at_one.to_str().unwrap()]);
        succeeded(&verified);
        assert!(String::from_utf8_lossy(&verified.stdout).contains(": ok (format 1, schema 1"));
    }
    let stderr = String::from_utf8_lossy(&one.stderr);
    assert!(stderr.contains("checkpoint alpha: recomputed (resume not requested)"));
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.path("qo-1/checkpoint-run.json")).unwrap())
            .unwrap();
    assert_eq!(record["groups"][1]["group"], "beta-group");
    // The invocation record names the checkpoints only when they are used.
    let invocation: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.path("one/report.invocation.json")).unwrap())
            .unwrap();
    assert_eq!(invocation["checkpoints"]["resume"], false);
    let invocation: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.path("whole/report.invocation.json")).unwrap())
            .unwrap();
    assert!(invocation.get("checkpoints").is_none());
}

#[test]
fn resume_reuses_exactly_the_groups_whose_identity_is_unchanged() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    succeeded(&fixture.run("first", &["--jobs", "2", "--checkpoint-dir", "qo"]));
    let alpha = fixture.path("qo/alpha.qo");
    let alpha_bytes = fs::read(&alpha).unwrap();
    // No change: both groups reused, the outputs unchanged, the files untouched.
    let again = fixture.run("again", &["--jobs", "2", "--checkpoint-dir", "qo", "--resume"]);
    succeeded(&again);
    let stderr = String::from_utf8_lossy(&again.stderr);
    assert!(stderr.contains("checkpoint alpha: reused (identity equal)"), "{stderr}");
    assert!(stderr.contains("checkpoint beta-group: reused (identity equal)"));
    assert_eq!(fixture.outputs("again"), fixture.outputs("first"));
    assert_eq!(fs::read(&alpha).unwrap(), alpha_bytes);
    // One group's source changes (uncommitted, so its unit is refused against
    // the pinned blob): that group alone is recomputed, and the result is the
    // whole run's over the changed input.
    fs::write(
        fixture.path("checkout/beta/beta_1.c"),
        "void beta_1(void) {}\n",
    )
    .unwrap();
    let changed = fixture.run("changed", &["--jobs", "2", "--checkpoint-dir", "qo", "--resume"]);
    succeeded(&changed);
    let stderr = String::from_utf8_lossy(&changed.stderr);
    assert!(stderr.contains("checkpoint alpha: reused (identity equal)"), "{stderr}");
    assert!(
        stderr.contains("checkpoint beta-group: recomputed (unit 1 (beta/beta_1.c): compile configuration differs)"),
        "{stderr}"
    );
    succeeded(&fixture.run("changed-whole", &["--jobs", "2"]));
    assert_eq!(fixture.outputs("changed"), fixture.outputs("changed-whole"));
    // A fresh checkpointed run over the changed input seals the same content
    // for both groups: a reused group equals a recomputed one.
    succeeded(&fixture.run("fresh", &["--jobs", "1", "--checkpoint-dir", "qo-fresh"]));
    for group in ["alpha", "beta-group"] {
        assert_eq!(
            content(&fixture.path(&format!("qo/{group}.qo"))),
            content(&fixture.path(&format!("qo-fresh/{group}.qo"))),
            "{group}"
        );
    }
    // A tampered checkpoint is never used: it is recomputed, with the reason.
    let mut tampered = fs::read(&alpha).unwrap();
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    fs::write(&alpha, &tampered).unwrap();
    let repaired = fixture.run("repaired", &["--jobs", "2", "--checkpoint-dir", "qo", "--resume"]);
    succeeded(&repaired);
    let stderr = String::from_utf8_lossy(&repaired.stderr);
    assert!(
        stderr.contains("checkpoint alpha: recomputed (checkpoint refused:"),
        "{stderr}"
    );
    assert_eq!(fixture.outputs("repaired"), fixture.outputs("changed-whole"));
    // A different contract set recomputes every group.
    fs::write(fixture.path("heap.qcore"), format!("{HEAP}\n")).unwrap();
    let contracts = fixture.run("contracts", &["--jobs", "2", "--checkpoint-dir", "qo", "--resume"]);
    succeeded(&contracts);
    let stderr = String::from_utf8_lossy(&contracts.stderr);
    assert!(
        stderr.contains("checkpoint alpha: recomputed (contract set differs)"),
        "{stderr}"
    );
    refused(&fixture.run("bad", &["--resume"]), "--resume needs --checkpoint-dir");
}

#[test]
fn a_selection_with_an_unrecorded_file_resumes_and_composes() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let selection = ["--selection", "selection.json", "--jobs", "2"];
    let whole = fixture.run("whole", &selection);
    succeeded(&whole);
    let mut checkpointed = selection.to_vec();
    checkpointed.extend(["--checkpoint-dir", "qo"]);
    succeeded(&fixture.run("first", &checkpointed));
    assert_eq!(fixture.outputs("first"), fixture.outputs("whole"));
    checkpointed.push("--resume");
    let again = fixture.run("again", &checkpointed);
    succeeded(&again);
    let stderr = String::from_utf8_lossy(&again.stderr);
    assert!(stderr.contains("checkpoint alpha: reused"), "{stderr}");
    assert!(stderr.contains("checkpoint beta-picked: reused"), "{stderr}");
    assert_eq!(fixture.outputs("again"), fixture.outputs("whole"));
}

#[test]
fn composition_is_the_whole_run_and_refuses_an_incomplete_or_mixed_set() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    let whole = fixture.run("whole", &["--jobs", "2"]);
    succeeded(&whole);
    succeeded(&fixture.run("ck", &["--jobs", "2", "--checkpoint-dir", "qo"]));
    let alpha = fixture.path("qo/alpha.qo");
    let beta = fixture.path("qo/beta-group.qo");
    let (alpha, beta) = (alpha.to_str().unwrap(), beta.to_str().unwrap());
    // Order of the arguments does not matter: the plan orders the groups.
    let composed = fixture.checkpoint(&[
        "compose",
        beta,
        alpha,
        "--report",
        "composed/report.json",
        "--monkey-source-report",
        "composed/monkey-source.json",
    ]);
    succeeded(&composed);
    assert_eq!(fixture.outputs("composed"), fixture.outputs("whole"));
    assert_eq!(console(&composed), console(&whole));
    refused(
        &fixture.checkpoint(&["compose", alpha, "--report", "x/report.json"]),
        "missing: [\"beta-group\"]",
    );
    refused(
        &fixture.checkpoint(&["compose", alpha, alpha, beta, "--report", "x/report.json"]),
        "is given twice",
    );
    assert!(!fixture.path("x/report.json").exists());
    // A checkpoint of another run (another contract set) is refused.
    fs::write(fixture.path("heap.qcore"), format!("{HEAP}\n")).unwrap();
    succeeded(&fixture.run("other", &["--jobs", "2", "--checkpoint-dir", "qo-other"]));
    let other_beta = fixture.path("qo-other/beta-group.qo");
    refused(
        &fixture.checkpoint(&[
            "compose",
            alpha,
            other_beta.to_str().unwrap(),
            "--report",
            "x/report.json",
        ]),
        "disagree on the contract set",
    );
    // A truncated checkpoint is refused whole, by verify and by compose.
    let bytes = fs::read(alpha).unwrap();
    fs::write(fixture.path("truncated.qo"), &bytes[..bytes.len() - 5]).unwrap();
    refused(
        &fixture.checkpoint(&["verify", "truncated.qo"]),
        "truncated",
    );
    refused(
        &fixture.checkpoint(&["compose", "truncated.qo", beta, "--report", "x/report.json"]),
        "truncated",
    );
    assert!(!fixture.path("x/report.json").exists());
}

#[test]
fn decomposition_round_trips_through_yaml_and_toml_to_the_same_bytes() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang);
    succeeded(&fixture.run("ck", &["--jobs", "2", "--checkpoint-dir", "qo"]));
    for group in ["alpha", "beta-group"] {
        let path = fixture.path(&format!("qo/{group}.qo"));
        let bytes = fs::read(&path).unwrap();
        let checkpoint = Checkpoint::from_bytes(&bytes).unwrap();
        for (format, name) in [(TextFormat::Yaml, "yaml"), (TextFormat::Toml, "toml")] {
            // In memory: text parses back to the same checkpoint and bytes.
            let text = decompose(&checkpoint, format).unwrap();
            let back = parse_text(&text, format).unwrap();
            assert_eq!(back, checkpoint);
            assert_eq!(back.to_bytes().unwrap(), bytes);
            // Through the command: decompose, then recompose, gives the file.
            let text_path = format!("{group}.{name}");
            succeeded(&fixture.checkpoint(&[
                "decompose",
                path.to_str().unwrap(),
                "--format",
                name,
                "--output",
                &text_path,
            ]));
            let recomposed = format!("{group}-{name}.qo");
            succeeded(&fixture.checkpoint(&[
                "recompose",
                &text_path,
                "--format",
                name,
                "--output",
                &recomposed,
            ]));
            assert_eq!(fs::read(fixture.path(&recomposed)).unwrap(), bytes);
        }
    }
}
