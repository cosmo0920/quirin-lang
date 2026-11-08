//! Card MEASURE-QUOTA-AND-INCREMENTAL, requirement R7
//! (`docs/plans/gate-time-reduction/README.md`): a measurement queued behind
//! a held gate slot is not killed for waiting. The slot is a real `flock`,
//! held by `scripts/gate_slot.py rust-suite` in a scratch gate directory (as
//! the script's own tests hold theirs), and the measurement runs through the
//! real host with a wall-time quota shorter than the wait.

#![cfg(unix)]

mod support;

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::Value;
use support::*;

const QUOTA_MS: &str = "4000";
const HOLD_SECONDS: &str = "6";

/// The host binary beside the plugin, built on demand as `tests/host.rs`
/// builds it (a scoped gate may build this crate alone).
fn host() -> PathBuf {
    let directory = Path::new(PLUGIN).parent().unwrap();
    let host = directory.join("quirin-audit");
    if !host.is_file() {
        let mut command = Command::new(env!("CARGO"));
        command.args([
            "build",
            "--offline",
            "-p",
            "qcore-audit",
            "--bin",
            "quirin-audit",
        ]);
        if directory.file_name().is_some_and(|name| name == "release") {
            command.arg("--release");
        }
        let status = command
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .unwrap();
        assert!(status.success(), "building the host failed");
    }
    host
}

fn repository_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/gate_slot.py")
}

/// The scratch gate's variables, for the holder, the host and (through
/// `--plugin-env`) every route's slot script.
fn gate_environment(directory: &Path) -> Vec<(String, String)> {
    let meminfo = directory.join("meminfo");
    // 8 GiB: one audit slot at the 7 GiB budget, a ceiling of two at 4 GiB.
    std::fs::write(
        &meminfo,
        "MemTotal:       8388608 kB\nMemFree:        1 kB\nMemAvailable:   8388608 kB\n",
    )
    .unwrap();
    vec![
        (
            "QUIRIN_GATE_DIR".into(),
            directory.join("gate").display().to_string(),
        ),
        (
            "QUIRIN_GATE_LEGACY_LOCK".into(),
            directory.join("legacy.lock").display().to_string(),
        ),
        ("QUIRIN_GATE_MEMINFO".into(), meminfo.display().to_string()),
    ]
}

/// A `rust-suite` job holding every audit slot for [`HOLD_SECONDS`]; returns
/// once it holds them.
fn hold_the_suite_slot(directory: &Path) -> Child {
    let mut holder = Command::new("python3")
        .arg(repository_script())
        .args(["rust-suite", "--receipt"])
        .arg(directory.join("holder.json"))
        .args(["--", "sleep", HOLD_SECONDS])
        .envs(gate_environment(directory))
        .env_remove("QUIRIN_GATE_HELD")
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(holder.stderr.take().unwrap()).lines();
    loop {
        let line = lines
            .next()
            .expect("the holder ends before taking its slot")
            .unwrap();
        if line.starts_with("gate_slot: rust-suite slot 0") {
            break;
        }
    }
    // Keep draining so the holder never blocks on a full pipe.
    std::thread::spawn(move || for _ in lines {});
    holder
}

/// A repository with the real slot script, the inputs, a recorded base side
/// and a stub analyzer for the tip.
fn fixture(name: &str) -> (PathBuf, String, String) {
    let directory = scratch(name);
    std::fs::create_dir_all(directory.join("scripts")).unwrap();
    std::fs::write(
        directory.join("scripts/gate_slot.py"),
        std::fs::read(repository_script()).unwrap(),
    )
    .unwrap();
    inputs(&directory);
    write_route(
        &directory.join("a"),
        "slice",
        &before_units(),
        Some("analyzer@1"),
    );
    write_route(
        &directory.join("fixture"),
        "report",
        &after_units(),
        Some("stub@1"),
    );
    let report = directory.join("fixture/report/resource-diagnostics.json");
    let binary = stub_checkout(&directory.join("stub"), &report, false);
    let manifest = manifest(&directory).display().to_string();
    (directory, manifest, format!("tip={}", binary.display()))
}

/// Run the measurement through the host; `limit` is the slot-wait limit, or
/// empty for the host's default.
fn measure(directory: &Path, manifest: &str, tip: &str, limit: &str) -> (bool, Value, Value) {
    let plugins = Path::new(PLUGIN).parent().unwrap().display().to_string();
    let limit_arguments: Vec<&str> = if limit.is_empty() {
        Vec::new()
    } else {
        vec!["--slot-wait-limit-ms", limit]
    };
    let result = Command::new(host())
        .args(["--plugin-path", &plugins, "--output", "out"])
        .args(["--timeout-ms", QUOTA_MS])
        .args(&limit_arguments)
        .args([
            "--plugin-env",
            "QUIRIN_GATE_DIR",
            "--plugin-env",
            "QUIRIN_GATE_LEGACY_LOCK",
            "--plugin-env",
            "QUIRIN_GATE_MEMINFO",
        ])
        .args(["measure", "--inputs", manifest, "--side", "base=run:a"])
        .args(["--side", tip, "--mount-root", "/tmp/quirin-measure-test"])
        .envs(gate_environment(directory))
        .env_remove("QUIRIN_AUDIT_PLUGIN_PATH")
        .env_remove("QUIRIN_GATE_HELD")
        .current_dir(directory)
        .output()
        .unwrap();
    let read = |name: &str| -> Value {
        std::fs::read(directory.join("out").join(name))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or(Value::Null)
    };
    (
        result.status.success(),
        read("quirin-audit-measure.report.json"),
        read("native-comparison.json"),
    )
}

#[test]
fn a_measurement_waiting_for_a_held_slot_is_not_killed_for_waiting() {
    host();
    // With the waits counted (`--slot-wait-limit-ms 0`, every run before
    // R7), the quota expires while the route waits: Unknown, killed.
    let (directory, manifest, tip) = fixture("slot-wait-counted");
    let mut holder = hold_the_suite_slot(&directory);
    let (completed, report, _) = measure(&directory, &manifest, &tip, "0");
    assert!(!completed, "{report:#}");
    assert_eq!(report["run"]["code"], "plugin-timeout", "{report:#}");
    assert_eq!(report["run"]["verdict"], "Unknown");
    assert!(holder.wait().unwrap().success());

    // With the waits outside the quota, the same measurement completes; the
    // wait is recorded apart from the analysis time, by the host, in the
    // record and per side and profile. The limit is the host's default, the
    // overall wall bound, and not the quota (GATE-SLOT-FIFO-ORDER).
    let (directory, manifest, tip) = fixture("slot-wait-excluded");
    let mut holder = hold_the_suite_slot(&directory);
    let (completed, report, record) = measure(&directory, &manifest, &tip, "");
    assert!(completed, "{report:#}");
    assert!(holder.wait().unwrap().success());
    let run = &report["run"];
    let wall = run["wall_time_ms"].as_u64().unwrap();
    let waited = run["slot_wait_ms"].as_u64().unwrap();
    assert!(waited >= 3000, "{run:#}");
    assert!(wall > QUOTA_MS.parse().unwrap(), "{run:#}");
    assert!(wall - waited < QUOTA_MS.parse().unwrap(), "{run:#}");
    let timing = &record["timing"];
    assert!(
        timing["slot_wait_ms"].as_u64().unwrap() >= 3000,
        "{timing:#}"
    );
    // Both limits are in the measure report's timing and in the host's records.
    let overall = quirin_audit_plugin::DEFAULT_WALL_LIMIT_MS;
    let limits = &timing["limits"];
    assert_eq!(limits["wall_time_quota_ms"], 4000, "{limits:#}");
    assert_eq!(limits["slot_wait_limit_ms"], overall, "{limits:#}");
    assert_eq!(limits["overall_wall_limit_ms"], overall, "{limits:#}");
    assert!(overall > 100 * 4000, "the default bound dwarfs the quota");
    assert_eq!(run["slot_wait_limit_ms"], overall, "{run:#}");
    assert_eq!(run["wall_limit_ms"], overall, "{run:#}");
    assert_eq!(run["wall_time_quota_ms"], 4000, "{run:#}");
    assert_eq!(report["request"]["quotas"]["wall_limit_ms"], overall);
    let route = &timing["sides"]["tip"]["slice"];
    assert_eq!(route["profile"], "fixture-profile");
    assert!(route["slot_wait_ms"].as_u64().unwrap() >= 3000, "{route:#}");
    assert_eq!(
        route["analysis_time_ms"].as_u64().unwrap(),
        route["wall_time_ms"].as_u64().unwrap() - route["slot_wait_ms"].as_u64().unwrap()
    );
    let waited_record: Value = serde_json::from_slice(
        &std::fs::read(directory.join("out/sides/tip/slice.slot-wait.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(waited_record["state"], "admitted");
    assert!(
        !directory
            .join("out")
            .join(quirin_audit_plugin::SLOT_WAIT_MARKER)
            .exists(),
        "the marker is gone once nothing waits"
    );
    // The comparison itself is the one any measurement makes.
    assert_eq!(record["status"], "complete");
}
