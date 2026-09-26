#!/usr/bin/env python3
"""Run the pinned 185-unit corpus audit once into a fresh private directory.

W45-FB-SYSTEM-SELECTION copy of docs/reports/w43-return-expression/scripts/run_audit.py
whose only change is the executable: ARBITER_BINARY names it (used for the
base-commit control run of the arbiter example built from 6bd9b383).

Usage: run_audit.py OUT_DIR SOURCE_COMMIT [--census] [--jobs N]

Reproduces the W42 command shape (`--pinned-evidence`, pinned Fluent Bit source
and Monkey compile database, `QCORE_CLANG=/usr/bin/clang`, the wave-20 ChunkIO
package) with the release binary of the current worktree. `--census` also sets
`QCORE_RETURN_CENSUS`; it has an effect only in a binary built with
`--features qcore-c-gateway/indirect-census`. The output directory must not
exist. Records `run.json` with the command, commit, binary digest, times and
exit; the audit's own stdout/stderr go to `audit.log`.

`--jobs N` (card W44-AUDIT-PARALLEL-UNITS) sets `QCORE_AUDIT_JOBS=N`, the
number of translation units the audit analyses concurrently; every artifact
and `audit.log` are byte-identical at every job count. Without it an inherited
`QCORE_AUDIT_JOBS` is used, else the binary's default (the host's available
parallelism, capped at 16). A census run always analyses one unit at a time.
`run.json`'s environment block records the value as `QCORE_AUDIT_JOBS`
(`"unset"` for the default); the command line is unchanged.
"""

import datetime
import hashlib
import json
import os
import pathlib
import subprocess
import sys

PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")
ROOT = None


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    commit = sys.argv[2]
    census = "--census" in sys.argv[3:]
    jobs = None
    if "--jobs" in sys.argv[3:]:
        jobs = sys.argv[sys.argv.index("--jobs", 3) + 1]
        if not jobs.isdigit() or int(jobs) < 1:
            raise SystemExit(f"--jobs needs a positive integer, not {jobs!r}")
    out.mkdir(parents=True, exist_ok=False)
    binary = pathlib.Path(os.environ["ARBITER_BINARY"]).resolve()
    command = [
        str(binary),
        "--pinned-evidence",
        str(PIN / "c-gateway-wave-pin/fluent-bit-clang-source"),
        str(PIN / "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"),
        str(out / "monkey-source.json"),
        str(out / "resource-diagnostics.json"),
    ]
    environment = dict(os.environ)
    environment["QCORE_CLANG"] = "/usr/bin/clang"
    environment["QCORE_CHUNKIO_PACKAGE"] = str(PIN / "provider-packages-wave20/qcore.chunkio")
    environment.pop("QCORE_SOURCE_CONTRACTS", None)
    if census:
        environment["QCORE_RETURN_CENSUS"] = str(out / "return-census.jsonl")
    if jobs is not None:
        environment["QCORE_AUDIT_JOBS"] = jobs
    record = {
        "command": command,
        "source_commit": commit,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "environment": {
            "QCORE_CLANG": environment["QCORE_CLANG"],
            "QCORE_CHUNKIO_PACKAGE": environment["QCORE_CHUNKIO_PACKAGE"],
            "QCORE_SOURCE_CONTRACTS": "unset",
            "QCORE_RETURN_CENSUS": environment.get("QCORE_RETURN_CENSUS", "unset"),
            "QCORE_AUDIT_JOBS": environment.get("QCORE_AUDIT_JOBS", "unset"),
        },
        "start": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    }
    with open(out / "audit.log", "wb") as log:
        exit_code = subprocess.run(command, env=environment, stdout=log, stderr=log).returncode
    record["exit"] = exit_code
    record["end"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
