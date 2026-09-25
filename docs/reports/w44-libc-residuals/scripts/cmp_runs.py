#!/usr/bin/env python3
"""Byte-compare the five raw artifacts of two audit runs.

Usage: cmp_runs.py LEFT_RUN RIGHT_RUN [OUT.json]

Prints, per artifact, both sha256 digests and whether they are identical.
Exits 0 only when every artifact is byte-identical.
"""

import hashlib
import json
import pathlib
import sys

NAMES = [
    "resource-diagnostics.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.source-effects.json",
    "monkey-source.json",
]


def main() -> int:
    left, right = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    rows = {}
    for name in NAMES:
        a = hashlib.sha256((left / name).read_bytes()).hexdigest()
        b = hashlib.sha256((right / name).read_bytes()).hexdigest()
        rows[name] = {"left": a, "right": b, "identical": a == b}
    record = {"left": str(left), "right": str(right), "artifacts": rows,
              "all_identical": all(r["identical"] for r in rows.values())}
    text = json.dumps(record, indent=1) + "\n"
    if len(sys.argv) > 3:
        pathlib.Path(sys.argv[3]).write_text(text)
    print(text)
    return 0 if record["all_identical"] else 1


if __name__ == "__main__":
    sys.exit(main())
