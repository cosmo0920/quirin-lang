#!/usr/bin/env python3
"""List every diagnostic that names one of the given callees, with its function.

Usage: callee_sites.py ARTIFACT_DIR NAME [NAME...]

Reads `diagnostics.json.gz` (or a run's `resource-diagnostics.json`) and prints
`unit::function status origin category description` for every diagnostic whose
description names NAME in backquotes.
"""

import gzip
import json
import pathlib
import sys


def load(root: pathlib.Path):
    if (root / "diagnostics.json.gz").exists():
        return json.load(gzip.open(root / "diagnostics.json.gz"))
    return json.loads((root / "resource-diagnostics.json").read_text())


def main() -> int:
    root = pathlib.Path(sys.argv[1])
    names = sys.argv[2:]
    data = load(root)
    for unit in data["units"]:
        for entry in unit["functions"]:
            for diag in entry.get("diagnostics") or []:
                for name in names:
                    if f"`{name}`" in diag["description"] or f"`qcore_" in diag["description"] and name in diag["description"]:
                        print(
                            f"{unit['file']}::{entry['function']} {entry.get('status')} "
                            f"{diag.get('origin')} {diag['category']} {diag['description']}"
                        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
