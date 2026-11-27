#!/usr/bin/env python3
"""CG11-POINTER-VALUED-STORES: CG00's frozen function list re-read on a run.

Usage: cg00_list.py RUN_DIR

For every function docs/reports/cg00-clang-gap-baseline/predictions/CG11.md
names as freed alone, print its status in RUN_DIR's resource-diagnostics.json
and its diagnostics (category, line, description), so a card can say which
predicted functions have already moved and what still blocks the rest.
"""

import collections
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]


def main() -> int:
    run = pathlib.Path(sys.argv[1])
    data = json.loads((run / "resource-diagnostics.json").read_text())
    rows = {}
    for unit in data["units"]:
        for function in unit["functions"]:
            rows[(unit["file"], function["function"])] = function
    by_suffix = {}
    for (unit, name), row in rows.items():
        by_suffix.setdefault(name, []).append((unit, row))
    text = (ROOT / "docs/reports/cg00-clang-gap-baseline/predictions/CG11.md").read_text()
    listed = re.findall(r"^- `([^`]+)::([^`]+)` — ([^:]+):", text, re.M)
    status = collections.Counter()
    for unit, name, predicted in listed:
        matches = [row for file, row in by_suffix.get(name, []) if file.endswith(unit)]
        if len(matches) != 1:
            print(f"{unit}::{name}: {len(matches)} rows (predicted {predicted})")
            status["unmatched"] += 1
            continue
        row = matches[0]
        status[row["status"]] += 1
        print(f"{unit}::{name}: {row['status']} (predicted {predicted})")
        for diagnostic in row.get("diagnostics", []):
            detail = diagnostic.get("description") or diagnostic.get("variable") or ""
            print(f"    {diagnostic.get('category')} @{diagnostic.get('line')}: {detail}")
    print(dict(status))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
