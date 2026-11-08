//! Card W45-AUDIT-SUBCOMMAND-PLUGINS: external `quirin-audit NAME`
//! subcommands, end to end through the `quirin-audit` binary, with shell
//! scripts as plugins.
//!
//! Every control of the card's brief: an external plugin runs and its
//! artifacts are recorded; built-in shadowing is refused; a missing plugin is
//! a usage error (and `PATH` is not searched); a version mismatch, a timeout
//! (with its process group killed), oversize output, a nonzero exit and
//! malformed JSON are each `Unknown`; an uncertified `Safe` and an uncertified
//! `Unsafe` claim are `Pending`; a failed plugin joined with a `Safe` host row
//! is `Unknown`; no claim lowers an existing `Unsafe`; and the environment does
//! not leak.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

// The plugin scripts are written by a separate process, so a sibling test's concurrent
// spawn cannot make its `execve` fail with `ETXTBSY`
// (`tests/support/process_fixture.rs`). Data files stay `fs::write`.
#[path = "../../../tests/support/process_fixture.rs"]
mod process_fixture;

const HOST: &str = env!("CARGO_BIN_EXE_quirin-audit");

// The runner isolates this binary from other test binaries. Also isolate its
// shell/plugin fixtures from each other: concurrent launches can consume the
// timeout control's 500 ms before its child PID is published on macOS.
static PROCESS_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    PROCESS_TESTS.lock().expect("a prior plugin test panicked")
}

fn scratch(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("subcommand-plugins")
        .join(format!("{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("plugins")).unwrap();
    std::fs::create_dir_all(directory.join("out")).unwrap();
    directory.canonicalize().unwrap()
}

/// A plugin `quirin-audit-NAME` in `directory/plugins`: it saves its request
/// to `request.json` in its working directory, then runs `body`.
fn plugin(directory: &Path, name: &str, body: &str) -> PathBuf {
    let path = directory
        .join("plugins")
        .join(format!("quirin-audit-{name}"));
    process_fixture::write_executable_mode(
        &path,
        format!("#!/bin/sh\ncat > request.json\n{body}\n").as_bytes(),
        0o755,
    );
    path
}

fn response(artifacts: &Value, claims: &Value) -> String {
    json!({
        "protocol": "quirin-audit-plugin",
        "protocol_version": 1,
        "outcome": "completed",
        "artifacts": artifacts,
        "diagnostics": [],
        "claims": claims,
    })
    .to_string()
}

/// `body` that prints `text` as the response (single-quoted for the shell).
fn emit(text: &str) -> String {
    assert!(!text.contains('\''));
    format!("printf '%s\\n' '{text}'")
}

fn claim(verdict: &str) -> Value {
    json!({
        "artifact": "unit.c", "property": "resource-custody",
        "semantic_profile": "test.v1", "scope": "whole unit", "bounds": "single path",
        "assumptions": {}, "evidence_version": 1, "verdict": verdict,
        "reason": "the fixture says so", "certificates": [],
    })
}

fn host(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(HOST)
        .args(arguments)
        .current_dir(directory)
        .env_remove("QUIRIN_AUDIT_PLUGIN_PATH")
        .output()
        .unwrap()
}

/// `quirin-audit --plugin-path plugins --output out [extra...] NAME [args...]`.
fn run(directory: &Path, extra: &[&str], name: &str, arguments: &[&str]) -> Output {
    let mut all = vec!["--plugin-path", "plugins", "--output", "out"];
    all.extend_from_slice(extra);
    all.push(name);
    all.extend_from_slice(arguments);
    host(directory, &all)
}

fn report(directory: &Path, name: &str) -> Value {
    let path = directory
        .join("out")
        .join(format!("quirin-audit-{name}.report.json"));
    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap()
}

fn invocation(directory: &Path, name: &str) -> Value {
    let path = directory
        .join("out")
        .join(format!("quirin-audit-{name}.report.invocation.json"));
    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn an_external_plugin_runs_and_its_artifacts_are_recorded() {
    let _serial = serial();
    let directory = scratch("runs");
    let content = "prepared\n";
    let digest = qcore_schemas::sha256_hex(content.as_bytes());
    let body = format!(
        "printf 'prepared\\n' > made.txt\npwd > cwd.txt\n{}",
        emit(&response(
            &json!([{"path": "made.txt", "sha256": digest, "kind": "example"}]),
            &json!([])
        ))
    );
    let script = plugin(&directory, "tool", &body);
    let output = run(&directory, &[], "tool", &["--flag", "a b", "--output"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let report = report(&directory, "tool");
    assert_eq!(report["run"]["outcome"], "completed");
    assert_eq!(report["artifacts"][0]["relative"], "made.txt");
    assert_eq!(report["artifacts"][0]["sha256"], digest);
    assert_eq!(
        report["verdict"],
        Value::Null,
        "no claim publishes no verdict"
    );
    let request: Value =
        serde_json::from_slice(&std::fs::read(directory.join("out/request.json")).unwrap())
            .unwrap();
    assert_eq!(request["arguments"], json!(["--flag", "a b", "--output"]));
    assert_eq!(request["protocol_version"], 1);
    let out = directory.join("out");
    assert_eq!(request["output"], out.display().to_string());
    assert_eq!(
        std::fs::read_to_string(out.join("cwd.txt")).unwrap().trim(),
        out.display().to_string(),
        "the working directory is the declared output"
    );
    let invocation = invocation(&directory, "tool");
    let script_digest = qcore_schemas::sha256_hex(&std::fs::read(&script).unwrap());
    assert_eq!(invocation["plugin"]["sha256"], script_digest);
    assert_eq!(
        invocation["plugin"]["executable"],
        script.display().to_string()
    );
    assert_eq!(invocation["plugin"]["protocol_version"], 1);
    assert_eq!(invocation["plugin"]["response_protocol_version"], 1);
    assert_eq!(invocation["plugin"]["exit_code"], 0);
    assert!(out.join("quirin-audit-tool.report.stderr.log").is_file());
}

#[test]
fn a_builtin_name_is_never_run_externally() {
    let _serial = serial();
    let directory = scratch("builtin");
    for name in ["plugin", "prepare", "current", "pinned"] {
        plugin(&directory, name, "touch \"$0.ran\"");
    }
    let output = run(&directory, &[], "plugin", &["views"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("built-in"), "{}", stderr(&output));
    // With the plugin directory on the variable, the built-in runs (and
    // refuses its own missing arguments), never the external executable.
    for name in ["plugin", "prepare", "pinned"] {
        let output = Command::new(HOST)
            .arg(name)
            .arg("--no-such-flag")
            .current_dir(&directory)
            .env("QUIRIN_AUDIT_PLUGIN_PATH", directory.join("plugins"))
            .output()
            .unwrap();
        assert_ne!(output.status.code(), Some(0));
        assert!(!stderr(&output).contains("external"), "{}", stderr(&output));
    }
    for name in ["plugin", "prepare", "current", "pinned"] {
        assert!(
            !directory
                .join("plugins")
                .join(format!("quirin-audit-{name}.ran"))
                .exists(),
            "the external `{name}` ran"
        );
    }
    let listed = host(&directory, &["--plugin-path", "plugins", "--list-plugins"]);
    assert_eq!(listed.status.code(), Some(0));
    let text = String::from_utf8_lossy(&listed.stdout);
    assert!(text.contains("plugin\t"), "{text}");
    assert!(text.contains("shadowed by a built-in; never run"), "{text}");
}

#[test]
fn a_missing_plugin_is_a_usage_error_and_path_is_not_searched() {
    let _serial = serial();
    let directory = scratch("missing");
    plugin(
        &directory,
        "onpath",
        &emit(&response(&json!([]), &json!([]))),
    );
    let output = Command::new(HOST)
        .args(["--output", "out", "onpath"])
        .current_dir(&directory)
        .env_remove("QUIRIN_AUDIT_PLUGIN_PATH")
        .env("PATH", directory.join("plugins"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let text = stderr(&output);
    assert!(text.contains("no subcommand `onpath`"), "{text}");
    assert!(text.contains("PATH is not searched"), "{text}");
    assert!(!directory.join("out/request.json").exists());
    let output = run(&directory, &[], "absent", &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains(&directory.join("plugins").display().to_string()),
        "the search path is listed: {}",
        stderr(&output)
    );
}

fn assert_unknown(directory: &Path, name: &str, output: &Output, code: &str) {
    assert_eq!(output.status.code(), Some(1), "{}", stderr(output));
    let report = report(directory, name);
    assert_eq!(report["run"]["outcome"], "failed");
    assert_eq!(report["run"]["code"], code, "{report:#}");
    assert_eq!(report["verdict"], "Unknown");
    assert_eq!(report["run"]["verdict"], "Unknown");
}

#[test]
fn a_version_mismatch_is_unknown_with_a_named_reason() {
    let _serial = serial();
    let directory = scratch("version");
    let text = response(&json!([]), &json!([]))
        .replace("\"protocol_version\":1", "\"protocol_version\":2");
    plugin(&directory, "old", &emit(&text));
    let output = run(&directory, &[], "old", &[]);
    assert_unknown(&directory, "old", &output, "protocol-version-mismatch");
}

/// Card MEASURE-QUOTA-AND-INCREMENTAL (R7): time the plugin declares as a
/// gate-slot wait (the marker in its output) is not counted against the
/// wall-time quota, up to the slot-wait limit; a limit of 0 counts every
/// wait, a limit below the wait still times out, and a stale marker left in
/// the output excuses nothing.
#[test]
fn declared_slot_waits_are_outside_the_wall_time_quota_up_to_the_limit() {
    let _serial = serial();
    let marker = quirin_audit_plugin::SLOT_WAIT_MARKER;
    let body = format!(
        "touch {marker}\nsleep 2\nrm {marker}\n{}",
        emit(&response(&json!([]), &json!([])))
    );
    let directory = scratch("slot-wait");
    plugin(&directory, "waiting", &body);
    let output = run(
        &directory,
        &["--timeout-ms", "1000", "--slot-wait-limit-ms", "10000"],
        "waiting",
        &[],
    );
    assert!(
        output.status.success(),
        "{}{}",
        stderr(&output),
        String::from_utf8_lossy(&output.stdout)
    );
    let run_record = &report(&directory, "waiting")["run"];
    assert_eq!(run_record["outcome"], "completed");
    let wall = run_record["wall_time_ms"].as_u64().unwrap();
    let waited = run_record["slot_wait_ms"].as_u64().unwrap();
    assert!(wall >= 2000 && waited >= 1500, "{run_record}");
    assert_eq!(run_record["analysis_time_ms"].as_u64(), Some(wall - waited));
    assert!(wall - waited < 1000, "{run_record}");
    assert_eq!(
        invocation(&directory, "waiting")["slot_wait_limit_ms"],
        10000
    );

    // By default the limit is the overall wall bound, not the quota (card
    // GATE-SLOT-FIFO-ORDER): a 1.5 s wait under a 500 ms quota, three times the
    // quota, completes. Both limits are in the request the plugin received, the
    // invocation record and the run record.
    let directory = scratch("slot-wait-default");
    plugin(
        &directory,
        "waiting",
        &format!(
            "touch {marker}\nsleep 1.5\nrm {marker}\n{}",
            emit(&response(&json!([]), &json!([])))
        ),
    );
    let output = run(&directory, &["--timeout-ms", "500"], "waiting", &[]);
    assert!(
        output.status.success(),
        "{}{}",
        stderr(&output),
        String::from_utf8_lossy(&output.stdout)
    );
    let bound = quirin_audit_plugin::DEFAULT_WALL_LIMIT_MS;
    let recorded = invocation(&directory, "waiting");
    assert_eq!(recorded["slot_wait_limit_ms"], bound);
    assert_eq!(recorded["wall_limit_ms"], bound);
    let run_record = &report(&directory, "waiting")["run"];
    assert!(
        run_record["slot_wait_ms"].as_u64().unwrap() >= 1200,
        "{run_record}"
    );
    assert_eq!(run_record["wall_limit_ms"], bound);
    assert_eq!(run_record["slot_wait_limit_ms"], bound);
    assert_eq!(run_record["wall_time_quota_ms"], 500);
    let sent: Value =
        serde_json::from_slice(&std::fs::read(directory.join("out/request.json")).unwrap())
            .unwrap();
    assert_eq!(sent["quotas"]["wall_limit_ms"], bound);
    assert_eq!(sent["quotas"]["slot_wait_limit_ms"], bound);
    assert_eq!(sent["quotas"]["wall_time_ms"], 500);

    // The same plugin with waits counted, as every run was before R7.
    let directory = scratch("slot-wait-counted");
    plugin(&directory, "waiting", &body);
    let output = run(
        &directory,
        &["--timeout-ms", "1000", "--slot-wait-limit-ms", "0"],
        "waiting",
        &[],
    );
    assert_unknown(&directory, "waiting", &output, "plugin-timeout");

    // A limit below the wait: the run stops at quota + limit.
    let directory = scratch("slot-wait-limited");
    plugin(&directory, "waiting", &body);
    let output = run(
        &directory,
        &["--timeout-ms", "1000", "--slot-wait-limit-ms", "300"],
        "waiting",
        &[],
    );
    assert_unknown(&directory, "waiting", &output, "plugin-timeout");
    let run_record = &report(&directory, "waiting")["run"];
    assert!(run_record["slot_wait_ms"].as_u64().unwrap() <= 300);
    assert!(
        run_record["reason"]
            .as_str()
            .unwrap()
            .contains("declared slot waits excluded, limit 300 ms"),
        "{run_record}"
    );

    // A marker the plugin did not write is removed before the spawn.
    let directory = scratch("slot-wait-stale");
    plugin(
        &directory,
        "idle",
        &format!("sleep 2\n{}", emit(&response(&json!([]), &json!([])))),
    );
    std::fs::write(directory.join("out").join(marker), "").unwrap();
    let output = run(&directory, &["--timeout-ms", "1000"], "idle", &[]);
    assert_unknown(&directory, "idle", &output, "plugin-timeout");
}

/// Card GATE-SLOT-FIFO-ORDER: the overall wall bound is separate from the
/// analysis quota. A plugin that declares waits for ever is stopped at the
/// bound with its own code; the bound is overridable, never below the quota,
/// and the quota alone still stops a plugin that analyses too long.
#[test]
fn the_overall_wall_bound_stops_a_run_that_declares_waits_for_ever() {
    let _serial = serial();
    let marker = quirin_audit_plugin::SLOT_WAIT_MARKER;
    let directory = scratch("wall-bound");
    plugin(&directory, "waiting", &format!("touch {marker}\nsleep 30"));
    let started = std::time::Instant::now();
    let output = run(
        &directory,
        &["--timeout-ms", "500", "--wall-limit-ms", "1500"],
        "waiting",
        &[],
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(20));
    assert_unknown(&directory, "waiting", &output, "plugin-wall-limit");
    let run_record = &report(&directory, "waiting")["run"];
    let wall = run_record["wall_time_ms"].as_u64().unwrap();
    assert!((1500..10_000).contains(&wall), "{run_record}");
    assert_eq!(run_record["wall_limit_ms"], 1500);
    assert!(run_record["analysis_time_ms"].as_u64().unwrap() < 500);
    let reason = run_record["reason"].as_str().unwrap();
    assert!(reason.contains("overall wall bound"), "{reason}");
    assert!(reason.contains("500 ms analysis quota"), "{reason}");
    assert_eq!(invocation(&directory, "waiting")["wall_limit_ms"], 1500);

    // The bound is a bound on the whole run: a slot-wait limit above it does
    // not extend it.
    let directory = scratch("wall-bound-over-limit");
    plugin(&directory, "waiting", &format!("touch {marker}\nsleep 30"));
    let output = run(
        &directory,
        &[
            "--timeout-ms",
            "500",
            "--wall-limit-ms",
            "1200",
            "--slot-wait-limit-ms",
            "100000000",
        ],
        "waiting",
        &[],
    );
    assert_unknown(&directory, "waiting", &output, "plugin-wall-limit");

    // A plugin that analyses (declares no wait) is stopped by the quota, not
    // by the larger bound.
    let directory = scratch("wall-bound-quota");
    plugin(&directory, "busy", "sleep 30");
    let output = run(
        &directory,
        &["--timeout-ms", "500", "--wall-limit-ms", "60000"],
        "busy",
        &[],
    );
    assert_unknown(&directory, "busy", &output, "plugin-timeout");

    // The bound is never below the quota: refused before anything runs.
    let directory = scratch("wall-bound-refused");
    plugin(
        &directory,
        "waiting",
        &emit(&response(&json!([]), &json!([]))),
    );
    let output = run(
        &directory,
        &["--timeout-ms", "5000", "--wall-limit-ms", "1000"],
        "waiting",
        &[],
    );
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("--wall-limit-ms"),
        "{}",
        stderr(&output)
    );
    assert!(!directory.join("out/request.json").exists());

    // A quota above the default bound raises the default to the quota.
    let directory = scratch("wall-bound-default-raised");
    plugin(
        &directory,
        "waiting",
        &emit(&response(&json!([]), &json!([]))),
    );
    let quota = quirin_audit_plugin::DEFAULT_WALL_LIMIT_MS + 1000;
    let output = run(
        &directory,
        &["--timeout-ms", &quota.to_string()],
        "waiting",
        &[],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(invocation(&directory, "waiting")["wall_limit_ms"], quota);
}

#[test]
fn a_timeout_is_unknown_and_kills_the_process_group() {
    let _serial = serial();
    let directory = scratch("timeout");
    // Consume stdin with shell builtins: launching `cat` before publishing
    // child.pid can spend the entire timeout in executable startup on macOS.
    // This control needs no saved request; the 500 ms quota stays unchanged.
    process_fixture::write_executable_mode(
        &directory.join("plugins/quirin-audit-slow"),
        format!(
            "#!/bin/sh\n{}\nsleep 30 &\necho $! > child.pid\nwait\n",
            process_fixture::READ_REQUEST
        )
        .as_bytes(),
        0o755,
    );
    let started = std::time::Instant::now();
    let output = run(&directory, &["--timeout-ms", "500"], "slow", &[]);
    assert!(started.elapsed() < std::time::Duration::from_secs(20));
    assert_unknown(&directory, "slow", &output, "plugin-timeout");
    let child = std::fs::read_to_string(directory.join("out/child.pid"))
        .expect("the timeout fixture must publish its child PID before teardown");
    let child: u32 = child.trim().parse().expect("a numeric child PID");
    assert!(child > 0);
    // `ps` works on Darwin as well as Linux. A zombie has been killed but
    // may not yet have been reaped by init; every other listed state is live.
    let state = Command::new("ps")
        .args(["-p", &child.to_string(), "-o", "stat="])
        .output()
        .expect("ps must inspect the plugin's child");
    assert!(
        matches!(state.status.code(), Some(0 | 1)) && state.stderr.is_empty(),
        "could not inspect child {child}: {state:?}"
    );
    let state = String::from_utf8(state.stdout).expect("ps state is UTF-8");
    let state = state.trim();
    assert!(
        state.is_empty() || state.starts_with('Z'),
        "the plugin's child {child} survived the group kill: {state}"
    );
}

#[test]
fn oversize_output_is_unknown() {
    let _serial = serial();
    let directory = scratch("oversize");
    plugin(&directory, "loud", "head -c 200000 /dev/zero | tr '\\0' x");
    let output = run(&directory, &["--stdout-limit-bytes", "1000"], "loud", &[]);
    assert_unknown(&directory, "loud", &output, "plugin-output-limit");
    plugin(
        &directory,
        "noisy",
        "head -c 200000 /dev/zero | tr '\\0' x >&2; sleep 5",
    );
    let output = run(&directory, &["--stderr-limit-bytes", "1000"], "noisy", &[]);
    assert_unknown(&directory, "noisy", &output, "plugin-stderr-limit");
}

#[test]
fn a_nonzero_exit_is_unknown() {
    let _serial = serial();
    let directory = scratch("exit");
    plugin(
        &directory,
        "crash",
        &format!(
            "{}\nexit 3",
            emit(&response(&json!([]), &json!([claim("Safe")])))
        ),
    );
    let output = run(&directory, &[], "crash", &[]);
    assert_unknown(&directory, "crash", &output, "plugin-exit");
    let report = report(&directory, "crash");
    assert_eq!(report["run"]["exit_code"], 3);
    assert_eq!(report["claims"][0]["joined"], false);
}

#[test]
fn malformed_json_is_unknown() {
    let _serial = serial();
    let directory = scratch("malformed");
    plugin(&directory, "garbled", "echo 'not json'");
    let output = run(&directory, &[], "garbled", &[]);
    assert_unknown(&directory, "garbled", &output, "malformed-response");
}

#[test]
fn an_artifact_outside_the_output_is_unknown() {
    let _serial = serial();
    let directory = scratch("outside");
    let digest = qcore_schemas::sha256_hex(b"x\n");
    let body = format!(
        "printf 'x\\n' > ../escaped.txt\n{}",
        emit(&response(
            &json!([{"path": "../escaped.txt", "sha256": digest, "kind": "k"}]),
            &json!([])
        ))
    );
    plugin(&directory, "escape", &body);
    let output = run(&directory, &[], "escape", &[]);
    assert_unknown(&directory, "escape", &output, "plugin-artifact");
}

#[test]
fn uncertified_claims_are_pending() {
    let _serial = serial();
    let directory = scratch("claims");
    for (name, verdict) in [("says-safe", "Safe"), ("says-unsafe", "Unsafe")] {
        plugin(
            &directory,
            name,
            &emit(&response(&json!([]), &json!([claim(verdict)]))),
        );
        let output = run(&directory, &[], name, &[]);
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        let report = report(&directory, name);
        assert_eq!(report["verdict"], "Pending", "{report:#}");
        assert_eq!(report["rows"][0]["artifact"], "unit.c");
        assert_eq!(report["rows"][0]["verdict"], "Pending");
        assert_eq!(report["claims"][0]["firewall"]["verdict"], "Pending");
        assert_eq!(
            report["reasons"][0]["code"], "plugin-claim-uncertified",
            "{report:#}"
        );
        let label = report["plugin"]["label"].as_str().unwrap();
        assert!(label.starts_with(&format!("quirin-audit-{name}@")));
        assert_eq!(report["claims"][0]["plugin"]["label"], label);
    }
    // A certificate reference changes nothing without a checker (QP016).
    let mut certified = claim("Safe");
    certified["certificates"] = json!([{"kind": "witness", "path": "w", "sha256": "0".repeat(64)}]);
    plugin(
        &directory,
        "cites",
        &emit(&response(&json!([]), &json!([certified]))),
    );
    let output = run(&directory, &[], "cites", &[]);
    assert_eq!(output.status.code(), Some(0));
    let report = report(&directory, "cites");
    assert_eq!(report["verdict"], "Pending");
    assert_eq!(report["claims"][0]["firewall"]["code"], "QP016");
}

fn audit_report(directory: &Path, status: &str) -> PathBuf {
    let path = directory.join(format!("audit-{status}.json"));
    let reason_code = if status == "Unsafe" {
        json!("residual-custody")
    } else {
        Value::Null
    };
    std::fs::write(
        &path,
        json!({"units": [{"library": "l", "file": "unit.c", "status": status,
                          "reason": if status == "Unsafe" { "a leak" } else { "" },
                          "reason_code": reason_code}]})
        .to_string(),
    )
    .unwrap();
    path
}

#[test]
fn a_failed_plugin_joined_with_safe_is_unknown() {
    let _serial = serial();
    let directory = scratch("failed-join");
    let safe = audit_report(&directory, "Safe");
    plugin(&directory, "fails", "exit 1");
    let output = run(
        &directory,
        &["--audit-report", safe.to_str().unwrap()],
        "fails",
        &[],
    );
    assert_unknown(&directory, "fails", &output, "plugin-exit");
    let report = report(&directory, "fails");
    assert_eq!(report["rows"][0]["host"]["verdict"], "Safe");
    assert_eq!(report["rows"][0]["verdict"], "Unknown");
}

#[test]
fn no_claim_lowers_an_existing_unsafe() {
    let _serial = serial();
    let directory = scratch("unsafe-host");
    let unsafe_report = audit_report(&directory, "Unsafe");
    for (name, verdict) in [("safe-on-bad", "Safe"), ("unknown-on-bad", "Unknown")] {
        plugin(
            &directory,
            name,
            &emit(&response(&json!([]), &json!([claim(verdict)]))),
        );
        let output = run(
            &directory,
            &["--audit-report", unsafe_report.to_str().unwrap()],
            name,
            &[],
        );
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        let report = report(&directory, name);
        assert_eq!(report["rows"][0]["host"]["verdict"], "Unsafe");
        assert_eq!(report["rows"][0]["verdict"], "Unsafe");
        assert_eq!(report["verdict"], "Unsafe");
    }
    // And a Safe host row with an uncertified Safe claim is Pending, not Safe.
    let safe = audit_report(&directory, "Safe");
    let output = run(
        &directory,
        &["--audit-report", safe.to_str().unwrap()],
        "safe-on-bad",
        &[],
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(report(&directory, "safe-on-bad")["verdict"], "Pending");
}

#[test]
fn the_environment_does_not_leak() {
    let _serial = serial();
    let directory = scratch("environment");
    let body = format!(
        "env > environment.txt\n{}",
        emit(&response(&json!([]), &json!([])))
    );
    plugin(&directory, "snoop", &body);
    let run_with = |extra: &[&str]| {
        let mut arguments = vec!["--plugin-path", "plugins", "--output", "out"];
        arguments.extend_from_slice(extra);
        arguments.push("snoop");
        let output = Command::new(HOST)
            .args(&arguments)
            .current_dir(&directory)
            .env("QUIRIN_TEST_SECRET", "leaked-value")
            .env("CARGO_REGISTRY_TOKEN", "leaked-token")
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        std::fs::read_to_string(directory.join("out/environment.txt")).unwrap()
    };
    let seen = run_with(&[]);
    assert!(!seen.contains("leaked-value"), "{seen}");
    assert!(!seen.contains("leaked-token"), "{seen}");
    assert!(!seen.contains("CARGO_"), "{seen}");
    assert!(
        seen.lines().any(|line| line == "PATH=/usr/bin:/bin"),
        "{seen}"
    );
    for line in seen.lines() {
        let name = line.split('=').next().unwrap();
        // `sh` itself sets PWD, OLDPWD, SHLVL and `_`.
        assert!(
            [
                "PATH", "HOME", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "TZ", "PWD", "OLDPWD",
                "SHLVL", "_"
            ]
            .contains(&name),
            "unexpected variable {name}"
        );
    }
    let first = invocation(&directory, "snoop");
    assert_eq!(first["environment"]["cleared"], true);
    let seen = run_with(&["--plugin-env", "QUIRIN_TEST_SECRET"]);
    assert!(seen.contains("QUIRIN_TEST_SECRET=leaked-value"), "{seen}");
    assert!(!seen.contains("leaked-token"), "{seen}");
    let second = invocation(&directory, "snoop");
    assert!(
        second["environment"]["passed"]
            .as_array()
            .unwrap()
            .contains(&json!("QUIRIN_TEST_SECRET"))
    );
}

#[test]
fn a_provider_needs_a_load_path_and_an_output_is_required() {
    let _serial = serial();
    let directory = scratch("inputs");
    plugin(&directory, "tool", &emit(&response(&json!([]), &json!([]))));
    let output = run(
        &directory,
        &["--provider", "quirin-provider-libc"],
        "tool",
        &[],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("--load-path"),
        "{}",
        stderr(&output)
    );
    let output = host(&directory, &["--plugin-path", "plugins", "tool"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("--output"), "{}", stderr(&output));
    assert!(!directory.join("out/request.json").exists());
}
