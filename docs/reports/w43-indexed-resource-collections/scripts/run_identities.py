#!/usr/bin/env python3
"""Pairwise raw SHA-256 identity of the five audit artifacts across runs.

Usage: run_identities.py RUNS_DIR OUT_JSON LEFT:RIGHT [LEFT:RIGHT ...]
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
    runs = pathlib.Path(sys.argv[1])
    out = {}
    for pair in sys.argv[3:]:
        left, right = pair.split(":")
        rows = {}
        for name in NAMES:
            a, b = runs / left / name, runs / right / name
            if not (a.exists() and b.exists()):
                continue
            ha = hashlib.sha256(a.read_bytes()).hexdigest()
            hb = hashlib.sha256(b.read_bytes()).hexdigest()
            rows[name] = {"left": ha, "right": hb, "identical": ha == hb}
        out[f"{left} vs {right}"] = rows
        print(pair, {name: row["identical"] for name, row in rows.items()})
    pathlib.Path(sys.argv[2]).write_text(json.dumps(out, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
