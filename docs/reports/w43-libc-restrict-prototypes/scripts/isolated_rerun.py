#!/usr/bin/env python3
"""Rerun one workspace test in isolation N times, each exit captured to a file.

Usage: isolated_rerun.py OUT_DIR N PACKAGE TEST_BINARY TEST_NAME

Used for the gate's one failing row, `removing_the_join_edge_is_reported_under_every_schedule`,
which the ledger already records as load-sensitive (TSAN-JOIN-EDGE-ROW-LOAD-SENSITIVE).
Writes OUT_DIR/run-<i>.log and OUT_DIR/run-<i>.exit and a summary.json; the
failed full-suite run is kept as the gate record, not replaced.
"""
import datetime
import json
import os
import pathlib
import subprocess
import sys


def main() -> int:
    out = pathlib.Path(sys.argv[1])
    count = int(sys.argv[2])
    package, binary, name = sys.argv[3:6]
    out.mkdir(parents=True, exist_ok=False)
    environment = dict(os.environ, CARGO_BUILD_JOBS="6", RUST_TEST_THREADS="2",
                       CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0")
    command = ["cargo", "test", "-p", package, "--test", binary, "--", "--exact", name]
    exits = []
    for index in range(1, count + 1):
        load = os.getloadavg()[0]
        with open(out / f"run-{index}.log", "wb") as log:
            code = subprocess.run(command, env=environment, stdout=log, stderr=subprocess.STDOUT).returncode
        (out / f"run-{index}.exit").write_text(f"{code}\n")
        exits.append({"run": index, "exit": code, "load_1min_at_start": round(load, 2),
                      "end": datetime.datetime.now(datetime.timezone.utc).isoformat()})
    (out / "summary.json").write_text(json.dumps({"command": command, "runs": exits}, indent=2) + "\n")
    print(json.dumps(exits))
    return max(e["exit"] for e in exits)


if __name__ == "__main__":
    sys.exit(main())
