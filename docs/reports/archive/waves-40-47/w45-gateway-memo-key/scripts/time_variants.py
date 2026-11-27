#!/usr/bin/env python3
"""Wall and CPU time of the pinned corpus arbiter for several saved binaries.

Usage: time_variants.py OUT_DIR ROUNDS JOBS NAME=BINARY...

Runs each binary ROUNDS times, interleaved (round 1 of every binary, then
round 2, ...), with exactly the command and environment of
`docs/reports/w43-return-expression/scripts/run_audit.py` (pinned evidence,
`QCORE_CLANG=/usr/bin/clang`, the wave-20 ChunkIO package,
`QCORE_SOURCE_CONTRACTS` unset, `QCORE_AUDIT_JOBS=JOBS`), each into a fresh
OUT_DIR/NAME-ROUND directory. Records wall seconds and the child's user and
system CPU seconds (`os.wait4`), which include its Clang subprocesses, in
OUT_DIR/timing.json. CPU time is far less sensitive than wall time to other
work on a shared host, but not insensitive to it (cache and frequency
effects). Exits 0 only when every run exits 0.
"""

import hashlib
import json
import os
import pathlib
import subprocess
import sys
import time

PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    rounds, jobs = int(sys.argv[2]), sys.argv[3]
    variants = [argument.split("=", 1) for argument in sys.argv[4:]]
    out.mkdir(parents=True, exist_ok=False)
    environment = dict(os.environ)
    environment["QCORE_CLANG"] = "/usr/bin/clang"
    environment["QCORE_CHUNKIO_PACKAGE"] = str(PIN / "provider-packages-wave20/qcore.chunkio")
    environment["QCORE_AUDIT_JOBS"] = jobs
    environment.pop("QCORE_SOURCE_CONTRACTS", None)
    environment.pop("QCORE_RETURN_CENSUS", None)
    runs = []
    for round_number in range(1, rounds + 1):
        for name, binary in variants:
            run = out / f"{name}-{round_number}"
            run.mkdir()
            command = [
                str(pathlib.Path(binary).resolve()),
                "--pinned-evidence",
                str(PIN / "c-gateway-wave-pin/fluent-bit-clang-source"),
                str(PIN / "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"),
                str(run / "monkey-source.json"),
                str(run / "resource-diagnostics.json"),
            ]
            with open(run / "audit.log", "wb") as log:
                start = time.monotonic()
                child = subprocess.Popen(command, env=environment, stdout=log, stderr=log)
                _, status, usage = os.wait4(child.pid, 0)
                wall = time.monotonic() - start
            runs.append(
                {
                    "variant": name,
                    "round": round_number,
                    "binary_sha256": hashlib.sha256(pathlib.Path(binary).read_bytes()).hexdigest(),
                    "exit": os.waitstatus_to_exitcode(status),
                    "wall_seconds": round(wall, 2),
                    "user_seconds": round(usage.ru_utime, 2),
                    "system_seconds": round(usage.ru_stime, 2),
                    "load_average_after": os.getloadavg(),
                }
            )
            print(json.dumps(runs[-1]), flush=True)
    (out / "timing.json").write_text(json.dumps({"jobs": jobs, "runs": runs}, indent=1) + "\n")
    return 0 if all(run["exit"] == 0 for run in runs) else 1


if __name__ == "__main__":
    sys.exit(main())
