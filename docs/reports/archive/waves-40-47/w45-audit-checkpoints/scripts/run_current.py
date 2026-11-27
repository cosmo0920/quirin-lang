#!/usr/bin/env python3
"""Run `quirin-audit current` over the pinned 185-unit corpus inputs, or compose
checkpoints, once, into a fresh private directory, measuring wall time and peak
resident set size.

Usage:
  run_current.py OUT_DIR [--jobs N] [--checkpoint-dir DIR [--resume]]
                 [--compile-database PATH]
  run_current.py OUT_DIR --compose QO_DIR

The corpus inputs are the corpus arbiter's (`docs/reports/w43-return-expression/
scripts/run_audit.py`): the pinned Fluent Bit checkout and its Clang database,
and the forced-Libevent Monkey database, with `QCORE_CLANG=/usr/bin/clang`. The
route is `quirin-audit current` (card W45-AUDIT-CHECKPOINTS checkpoints that
route); the report and its sidecars are written under the names the arbiter
uses, so `baseline_identity.py` compares them. `--compile-database` substitutes
the main database (the one-group-change measurement). `--compose` instead runs
`quirin-audit checkpoint compose QO_DIR/*.qo` into OUT_DIR.

The release binary of this worktree runs under `/usr/bin/time -v`; `run.json`
records the command, the binary digest, the environment, the start and end,
the exit, the wall time and the peak RSS (KiB). The audit's own stdout and
stderr go to `audit.log`. OUT_DIR must not exist.
"""

import datetime
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin")
ROOT = pathlib.Path(__file__).resolve().parents[4]


def option(arguments, flag):
    if flag in arguments:
        return arguments[arguments.index(flag) + 1]
    return None


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    arguments = sys.argv[2:]
    binary = ROOT / "target/release/quirin-audit"
    out.mkdir(parents=True, exist_ok=False)
    compose = option(arguments, "--compose")
    if compose is not None:
        qo = sorted(str(path) for path in pathlib.Path(compose).glob("*.qo"))
        command = [str(binary), "checkpoint", "compose", *qo,
                   "--report", str(out / "resource-diagnostics.json"),
                   "--monkey-source-report", str(out / "monkey-source.json")]
    else:
        command = [
            str(binary), "current",
            "--checkout", str(PIN / "fluent-bit-clang-source"),
            "--profile-directory", str(ROOT / "contract/profiles/fluent-bit"),
            "--monkey-database", str(PIN / "monkey/monkey-build/compile_commands.json"),
            "--monkey-source-report", str(out / "monkey-source.json"),
            "--report", str(out / "resource-diagnostics.json"),
        ]
        database = option(arguments, "--compile-database")
        if database is not None:
            command += ["--compile-database", str(pathlib.Path(database).resolve())]
        jobs = option(arguments, "--jobs")
        if jobs is not None:
            command += ["--jobs", jobs]
        checkpoints = option(arguments, "--checkpoint-dir")
        if checkpoints is not None:
            command += ["--checkpoint-dir", str(pathlib.Path(checkpoints).resolve())]
            if "--resume" in arguments:
                command.append("--resume")
    environment = dict(os.environ)
    environment["QCORE_CLANG"] = "/usr/bin/clang"
    environment.pop("QCORE_SOURCE_CONTRACTS", None)
    environment.pop("QCORE_AUDIT_JOBS", None)
    record = {
        "command": command,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "environment": {"QCORE_CLANG": environment["QCORE_CLANG"],
                        "QCORE_AUDIT_JOBS": "unset"},
        "start": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    }
    timing = out / "time.txt"
    with open(out / "audit.log", "wb") as log:
        exit_code = subprocess.run(["/usr/bin/time", "-v", "-o", str(timing), *command],
                                   env=environment, stdout=log, stderr=log).returncode
    record["exit"] = exit_code
    record["end"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    text = timing.read_text()
    peak = re.search(r"Maximum resident set size \(kbytes\): (\d+)", text)
    wall = re.search(r"Elapsed \(wall clock\) time \(h:mm:ss or m:ss\): (\S+)", text)
    record["peak_rss_kib"] = int(peak.group(1)) if peak else None
    record["wall_clock"] = wall.group(1) if wall else None
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps({key: record[key] for key in ("exit", "peak_rss_kib", "wall_clock")}))
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
