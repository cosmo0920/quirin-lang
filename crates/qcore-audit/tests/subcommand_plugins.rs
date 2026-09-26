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

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

const HOST: &str = env!("CARGO_BIN_EXE_quirin-audit");

fn scratch(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("subcommand-plugins")
        .join(name);
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
    std::fs::write(&path, format!("#!/bin/sh\ncat > request.json\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
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
    let directory = scratch("builtin");
    for name in ["plugin", "prepare", "current"] {
        plugin(&directory, name, "touch \"$0.ran\"");
    }
    let output = run(&directory, &[], "plugin", &["views"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("built-in"), "{}", stderr(&output));
    // With the plugin directory on the variable, the built-in runs (and
    // refuses its own missing arguments), never the external executable.
    for name in ["plugin", "prepare"] {
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
    for name in ["plugin", "prepare", "current"] {
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
    let directory = scratch("version");
    let text = response(&json!([]), &json!([]))
        .replace("\"protocol_version\":1", "\"protocol_version\":2");
    plugin(&directory, "old", &emit(&text));
    let output = run(&directory, &[], "old", &[]);
    assert_unknown(&directory, "old", &output, "protocol-version-mismatch");
}

#[test]
fn a_timeout_is_unknown_and_kills_the_process_group() {
    let directory = scratch("timeout");
    plugin(
        &directory,
        "slow",
        "sleep 30 &\necho $! > child.pid\nsleep 30",
    );
    let started = std::time::Instant::now();
    let output = run(&directory, &["--timeout-ms", "500"], "slow", &[]);
    assert!(started.elapsed() < std::time::Duration::from_secs(20));
    assert_unknown(&directory, "slow", &output, "plugin-timeout");
    let child = std::fs::read_to_string(directory.join("out/child.pid")).unwrap();
    let child = child.trim();
    // The background `sleep` was in the plugin's process group, so it is gone.
    let alive = Path::new("/proc").join(child).exists()
        && std::fs::read_to_string(Path::new("/proc").join(child).join("stat"))
            .is_ok_and(|stat| !stat.contains(") Z "));
    assert!(!alive, "the plugin's child {child} survived the group kill");
}

#[test]
fn oversize_output_is_unknown() {
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
    let directory = scratch("malformed");
    plugin(&directory, "garbled", "echo 'not json'");
    let output = run(&directory, &[], "garbled", &[]);
    assert_unknown(&directory, "garbled", &output, "malformed-response");
}

#[test]
fn an_artifact_outside_the_output_is_unknown() {
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
