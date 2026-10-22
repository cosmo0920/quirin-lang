#!/usr/bin/env python3
"""Measure one configured resource audit in a dedicated systemd user cgroup.

Build/setup is excluded. All audit workers and descendants inherit the limit.
Requires Linux cgroup v2 and a systemd user manager; unavailable accounting is
an error, never a substitution of per-process RSS for a whole-job peak.
"""
import argparse
import hashlib
import json
import os
import signal
from pathlib import Path
import subprocess
import sys
import time
import uuid


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def sha256(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def worker(args):
    relative = Path("/proc/self/cgroup").read_text().strip().split("::", 1)[1]
    cgroup = Path("/sys/fs/cgroup") / relative.lstrip("/")
    start = time.monotonic()
    binary_sha = sha256(args.binary)
    config_sha = sha256(args.config)
    argv = ([str(args.binary), str(args.config)] if args.route == "configured" else
            [str(args.binary), "current", "--config", str(args.config)])
    argv += ["--jobs", str(args.jobs)]
    environment = dict(os.environ)
    # This measurement selects the configuration's resource audit only.
    for name in ["QCORE_AUDIT_IR_DIR", "QCORE_BOUNDS_PROPERTY", "QCORE_NULL_PROPERTY"]:
        environment.pop(name, None)
    with (args.output / "audit.log").open("x") as output:
        result = subprocess.run(argv, env=environment, stdout=output, stderr=subprocess.STDOUT)
    save(args.output / "worker.json", {
        "argv": argv, "exit": result.returncode,
        "elapsed_seconds": round(time.monotonic() - start, 3),
        "binary_sha256": binary_sha, "configuration_sha256": config_sha,
        "inputs_unchanged": binary_sha == sha256(args.binary) and config_sha == sha256(args.config),
        "cgroup": relative,
        "observed_before_exit": {name: (cgroup / name).read_text().strip() for name in
                                 ["memory.max", "memory.swap.max", "memory.peak", "memory.events"]},
        "environment": {name: environment.get(name) for name in
                        ["PATH", "QCORE_CLANG", "QCORE_AUDIT_IR_DIR", "QCORE_BOUNDS_PROPERTY", "QCORE_NULL_PROPERTY"]},
    })
    return result.returncode if result.returncode >= 0 else 128 - result.returncode


def measure(args):
    args.output.mkdir(parents=True, exist_ok=False)
    unit = "quirin-audit-memory-" + uuid.uuid4().hex
    command = [
        "systemd-run", "--user", "--unit=" + unit,
        "--property=MemoryAccounting=yes", "--property=MemoryMax=" + str(args.limit_bytes),
        "--property=MemorySwapMax=0", "--property=RemainAfterExit=yes",
        "--working-directory=" + str(Path.cwd()), "--setenv=PATH=" + os.environ["PATH"],
    ]
    if "QCORE_CLANG" in os.environ:
        command += ["--setenv=QCORE_CLANG=" + os.environ["QCORE_CLANG"]]
    command += [sys.executable, str(Path(__file__).resolve()), "--worker",
                "--binary", str(args.binary), "--config", str(args.config),
                "--output", str(args.output), "--jobs", str(args.jobs), "--route", args.route]
    start = time.monotonic()
    with (args.output / "service.log").open("x") as log:
        launched = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
    save(args.output / "launch.json", {"argv": command, "exit": launched.returncode})
    if launched.returncode:
        return launched.returncode
    properties = ["ActiveState", "SubState", "ExecMainCode", "ExecMainStatus", "Result",
                  "MemoryPeak", "MemorySwapPeak", "ControlGroup", "CPUUsageNSec",
                  "ExecMainStartTimestampMonotonic", "ExecMainExitTimestampMonotonic"]
    values = {}
    try:
        while True:
            shown = subprocess.run(["systemctl", "--user", "show", unit, "--no-pager",
                                    *["--property=" + name for name in properties]],
                                   text=True, capture_output=True)
            if shown.returncode:
                raise RuntimeError(shown.stderr)
            values = dict(line.split("=", 1) for line in shown.stdout.splitlines() if "=" in line)
            if values.get("SubState") in ["exited", "failed", "dead"]:
                break
            if time.monotonic() - start > args.timeout:
                raise TimeoutError("audit service exceeded its measurement deadline")
            time.sleep(2)
        path = args.output / "worker.json"
        observed = json.loads(path.read_text()) if path.exists() else None
        peak = values.get("MemoryPeak", "")
        peak = int(peak) if peak.isdigit() and 0 < int(peak) < 2**63 else None
        events = ({line.split()[0]: int(line.split()[1]) for line in
                   observed["observed_before_exit"]["memory.events"].splitlines()} if observed else None)
        limit = int(observed["observed_before_exit"]["memory.max"]) if observed else None
        complete = bool(observed and observed["exit"] == 0 and observed["inputs_unchanged"]
                        and values.get("Result") == "success" and values.get("SubState") == "exited"
                        and peak is not None and limit <= args.limit_bytes and peak <= limit
                        and events["oom"] == 0 and events["oom_kill"] == 0)
        record = {"service": unit, "service_properties": values, "worker": observed,
                  "memory_peak_bytes": peak, "memory_events": events,
                  "requested_limit_bytes": args.limit_bytes,
                  "audit_completed_without_oom": complete,
                  "monitored_elapsed_seconds": round(time.monotonic() - start, 3),
                  "scope": "Configured resource audit, existing build/setup excluded; fresh process caches, host page-cache state uncontrolled."}
        save(args.output / "result.json", record)
        print(json.dumps({key: record[key] for key in
                          ["memory_peak_bytes", "audit_completed_without_oom", "monitored_elapsed_seconds"]}), flush=True)
        return 0 if complete else 1
    except BaseException as error:
        save(args.output / "measurement-error.json", {"error": str(error), "service_properties": values})
        raise
    finally:
        stopped = subprocess.run(["systemctl", "--user", "stop", unit], text=True, capture_output=True)
        cleanup = {"stop_exit": stopped.returncode, "stop_stderr": stopped.stderr}
        if values.get("Result") != "success":
            reset = subprocess.run(["systemctl", "--user", "reset-failed", unit], text=True, capture_output=True)
            cleanup.update(reset_failed_exit=reset.returncode, reset_failed_stderr=reset.stderr)
        save(args.output / "cleanup.json", cleanup)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--route", choices=["configured", "current"], default="configured")
    parser.add_argument("--limit-bytes", type=int, default=7_000_000_000)
    parser.add_argument("--timeout", type=int, default=1800)
    parser.add_argument("--worker", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if min(args.jobs, args.limit_bytes, args.timeout) <= 0:
        parser.error("jobs, limit and deadline must be positive")
    args.binary = args.binary.resolve(strict=True)
    args.config = args.config.resolve(strict=True)
    args.output = args.output.resolve()
    if not args.worker:
        def terminate(number, _frame):
            raise SystemExit(128 + number)
        signal.signal(signal.SIGTERM, terminate)
    return worker(args) if args.worker else measure(args)


if __name__ == "__main__":
    sys.exit(main())
