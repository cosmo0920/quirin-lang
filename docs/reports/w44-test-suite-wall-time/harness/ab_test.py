#!/usr/bin/env python3
"""Alternate two builds of one test binary on one test and time each run.

Usage: ab_test.py <cwd> <test-name> <rounds> <label>=<executable> ...
Prints wall seconds and the 1-minute load average before each run.
Measurement helper only (card W44-TEST-SUITE-WALL-TIME).
"""

import os
import subprocess
import sys
import time


def main() -> int:
    cwd, name, rounds = sys.argv[1], sys.argv[2], int(sys.argv[3])
    variants = [arg.split("=", 1) for arg in sys.argv[4:]]
    for round_ in range(rounds):
        for label, exe in variants:
            load = os.getloadavg()[0]
            t0 = time.monotonic()
            proc = subprocess.run([os.path.abspath(exe), "--exact", name], cwd=cwd,
                                  capture_output=True, text=True)
            seconds = time.monotonic() - t0
            print(f"round {round_} {label:12} {seconds:7.1f}s exit {proc.returncode} load {load:.1f}",
                  flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
