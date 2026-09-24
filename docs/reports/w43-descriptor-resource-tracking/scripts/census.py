#!/usr/bin/env python3
"""Declared-ABI census of this card's declarations over the captured units.

Usage: census.py MODULE.qcore... OUT.txt

Runs `target/release/examples/declared_abi_census MODULE... -- UNIT.i...` over
the line-marker-stripped captures in `measure/capture/plain/` (see
`capture_run.sh`, `clang-tee.sh`, `strip.py`) and writes its rows to OUT.txt,
then prints per-name outcome counts. The tool is the production decision
(`legacy_c::declared_abi_census`), not a second implementation.
"""

import collections
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]


def main() -> int:
    *modules, out = sys.argv[1:]
    units = sorted((ROOT / "measure/capture/plain").glob("*.i"))
    command = [str(ROOT / "target/release/examples/declared_abi_census"), *modules, "--"]
    command += [str(unit) for unit in units]
    completed = subprocess.run(command, capture_output=True, text=True)
    pathlib.Path(out).write_text(completed.stdout + completed.stderr)
    per_name = collections.defaultdict(collections.Counter)
    for line in completed.stdout.splitlines():
        parts = line.split("\t")
        if len(parts) == 3:
            per_name[parts[1]][parts[2]] += 1
    print(f"units {len(units)} exit {completed.returncode}")
    for name, counts in sorted(per_name.items()):
        print(f"{name:12} {dict(counts)}")
    return completed.returncode


if __name__ == "__main__":
    sys.exit(main())
