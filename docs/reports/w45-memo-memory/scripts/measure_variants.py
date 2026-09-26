#!/usr/bin/env python3
"""Peak memory, wall and CPU time of saved analyser binaries on the pinned corpus.

Usage:
  measure_variants.py OUT_DIR ROUNDS JOBS arbiter NAME=CYCLE19_BINARY...
  measure_variants.py OUT_DIR ROUNDS JOBS current-ck NAME=QUIRIN_AUDIT_BINARY...

Runs each binary ROUNDS times, interleaved (round 1 of every binary, then
round 2, ...), each into a fresh OUT_DIR/NAME-ROUND directory. OUT_DIR must not
exist.

* `arbiter` is exactly the command and environment of
  `docs/reports/w43-return-expression/scripts/run_audit.py` (pinned evidence,
  `QCORE_CLANG=/usr/bin/clang`, the wave-20 ChunkIO package,
  `QCORE_SOURCE_CONTRACTS` unset, `QCORE_AUDIT_JOBS=JOBS`).
* `current-ck` is `docs/reports/w45-audit-checkpoints/scripts/run_current.py`'s
  checkpointed `quirin-audit current` (`--jobs JOBS --checkpoint-dir`), with the
  profile of this worktree.

Recorded per run in OUT_DIR/timing.json:

* `wall_seconds`, and the child's `user_seconds`/`system_seconds` (`os.wait4`),
  which include its Clang subprocesses;
* `tree_peak_kib`: `ru_maxrss` from `wait4`, the larger of the analyser's own
  peak and its largest waited descendant's (what `/usr/bin/time -v` prints);
* `self_peak_kib`: the analyser's own `VmHWM` (kernel high-water mark of its
  resident set), sampled from `/proc/PID/status` every 50 ms; the last sample
  before exit. It excludes Clang and can miss growth in the last 50 ms only.

Exits 0 only when every run exits 0.
"""

import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys
import time

PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")
ROOT = pathlib.Path(__file__).resolve().parents[4]


def command_for(route, binary, run, jobs):
    if route == "arbiter":
        return [
            binary,
            "--pinned-evidence",
            str(PIN / "c-gateway-wave-pin/fluent-bit-clang-source"),
            str(PIN / "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"),
            str(run / "monkey-source.json"),
            str(run / "resource-diagnostics.json"),
        ]
    if route == "current-ck":
        pin = PIN / "c-gateway-wave-pin"
        return [
            binary, "current",
            "--checkout", str(pin / "fluent-bit-clang-source"),
            "--profile-directory", str(ROOT / "contract/profiles/fluent-bit"),
            "--monkey-database", str(pin / "monkey/monkey-build/compile_commands.json"),
            "--monkey-source-report", str(run / "monkey-source.json"),
            "--report", str(run / "resource-diagnostics.json"),
            "--jobs", jobs,
            "--checkpoint-dir", str(run / "qo"),
        ]
    raise SystemExit(f"unknown route {route!r}")


def hwm_kib(pid):
    try:
        text = pathlib.Path(f"/proc/{pid}/status").read_text()
    except OSError:
        return None
    found = re.search(r"^VmHWM:\s+(\d+) kB", text, re.MULTILINE)
    return int(found.group(1)) if found else None


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    rounds, jobs, route = int(sys.argv[2]), sys.argv[3], sys.argv[4]
    variants = [argument.split("=", 1) for argument in sys.argv[5:]]
    out.mkdir(parents=True, exist_ok=False)
    environment = dict(os.environ)
    environment["QCORE_CLANG"] = "/usr/bin/clang"
    environment.pop("QCORE_SOURCE_CONTRACTS", None)
    environment.pop("QCORE_RETURN_CENSUS", None)
    if route == "arbiter":
        environment["QCORE_CHUNKIO_PACKAGE"] = str(PIN / "provider-packages-wave20/qcore.chunkio")
        environment["QCORE_AUDIT_JOBS"] = jobs
    else:
        environment.pop("QCORE_AUDIT_JOBS", None)
    runs = []
    for round_number in range(1, rounds + 1):
        for name, binary in variants:
            binary = str(pathlib.Path(binary).resolve())
            run = out / f"{name}-{round_number}"
            run.mkdir()
            command = command_for(route, binary, run, jobs)
            with open(run / "audit.log", "wb") as log:
                start = time.monotonic()
                child = subprocess.Popen(command, env=environment, stdout=log, stderr=log)
                self_peak = None
                while True:
                    pid, status, usage = os.wait4(child.pid, os.WNOHANG)
                    if pid != 0:
                        break
                    sample = hwm_kib(child.pid)
                    if sample is not None:
                        self_peak = sample
                    time.sleep(0.05)
                wall = time.monotonic() - start
            child.returncode = os.waitstatus_to_exitcode(status)
            runs.append(
                {
                    "variant": name,
                    "round": round_number,
                    "route": route,
                    "jobs": jobs,
                    "binary_sha256": hashlib.sha256(pathlib.Path(binary).read_bytes()).hexdigest(),
                    "exit": child.returncode,
                    "wall_seconds": round(wall, 2),
                    "user_seconds": round(usage.ru_utime, 2),
                    "system_seconds": round(usage.ru_stime, 2),
                    "tree_peak_kib": usage.ru_maxrss,
                    "self_peak_kib": self_peak,
                    "load_average_after": os.getloadavg(),
                }
            )
            print(json.dumps(runs[-1]), flush=True)
    (out / "timing.json").write_text(
        json.dumps({"jobs": jobs, "route": route, "runs": runs}, indent=1) + "\n"
    )
    return 0 if all(run["exit"] == 0 for run in runs) else 1


if __name__ == "__main__":
    sys.exit(main())
