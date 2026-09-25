#!/usr/bin/env python3
"""Time every test of the given test binaries, one process per test.

Usage: per_test_times.py --jobs N --out FILE <package-dir>=<executable> ...

Each test is run as `<executable> --exact <name>` with cwd = the package dir,
so a binary's wall time under `cargo test` is bounded below by its longest
test. Measurement helper only (card W44-TEST-SUITE-WALL-TIME).
"""

import argparse
import concurrent.futures
import json
import os
import subprocess
import time


def tests_of(exe, cwd):
    out = subprocess.run([exe, "--list", "--format", "terse"], cwd=cwd, capture_output=True,
                         text=True, check=True).stdout
    return [line[: -len(": test")] for line in out.splitlines() if line.endswith(": test")]


def run_one(exe, cwd, name):
    t0 = time.monotonic()
    proc = subprocess.run([exe, "--exact", name], cwd=cwd, capture_output=True, text=True)
    return {"binary": os.path.basename(exe), "test": name, "seconds": round(time.monotonic() - t0, 2),
            "exit": proc.returncode}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--out", required=True)
    parser.add_argument("pairs", nargs="+")
    args = parser.parse_args()
    work = []
    for pair in args.pairs:
        cwd, exe = pair.split("=", 1)
        exe = os.path.abspath(exe)
        for name in tests_of(exe, cwd):
            work.append((exe, cwd, name))
    with concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
        results = list(pool.map(lambda w: run_one(*w), work))
    results.sort(key=lambda r: -r["seconds"])
    with open(args.out, "w") as handle:
        json.dump(results, handle, indent=1)
    for r in results:
        print(f"{r['seconds']:8.2f}s exit {r['exit']} {r['binary']} {r['test']}")


if __name__ == "__main__":
    main()
