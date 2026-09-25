#!/usr/bin/env python3
"""Run several arbiter stages concurrently with `run_binary.py`.

Usage: run_stages_v2.py OUT_ROOT NAME=LABEL=BINARY [NAME=LABEL=BINARY ...]

Every stage reads the same contract files, which are the working tree's at the
time of the call; each run records their digests. Exits with the worst exit.
"""

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
FILES = [
    "cycle19-resource-contracts.json",
    "providers/contract-load-path.json",
    "providers/quirin-provider-libc/contracts/libc/stdio.qcore",
    "providers/quirin-provider-libc/contracts/libc/errno.qcore",
]


def main() -> int:
    root = pathlib.Path(sys.argv[1])
    processes = []
    for spec in sys.argv[2:]:
        name, label, binary = spec.split("=", 2)
        log = open(ROOT / "measure/logs" / f"v2-{name}.log", "wb")
        command = [sys.executable, str(ROOT / "docs/reports/w44-libc-residuals/scripts/run_binary.py"),
                   str(root / name), label, binary, *FILES]
        processes.append((name, subprocess.Popen(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)))
    worst = 0
    for name, process in processes:
        code = process.wait()
        (ROOT / "measure/logs" / f"v2-{name}.exit").write_text(f"{code}\n")
        worst = worst or code
    return worst


if __name__ == "__main__":
    sys.exit(main())
