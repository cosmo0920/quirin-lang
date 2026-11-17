#!/usr/bin/env python3
"""compare_base_tip.py BASE_RUN TIP_RUN OUT: compare two callers.sh runs (the
base and the tip binary) side by side, row by row: every function row whose
status or diagnostics differ, and every unit whose status differs. Writes
OUT (JSON) and prints it. Exit 1 if a row is unmatched between the runs."""
import json
import pathlib
import sys

base, tip, out = (pathlib.Path(arg) for arg in sys.argv[1:4])
SIDES = ("without", "with-selected", "with-default", "with-full")


def load(run, side):
    report = json.loads((run / side / "resource-diagnostics.json").read_text())
    rows = {(unit["file"], row["function"]): (row["status"],
                                              sorted(d["description"] for d in row["diagnostics"]))
            for unit in report["units"] for row in unit["functions"]}
    units = {unit["file"]: unit["status"] for unit in report["units"]}
    return rows, units


result = {"schema": "library-version-hold-base-tip/1", "sides": {}}
unmatched = False
for side in SIDES:
    base_rows, base_units = load(base, side)
    tip_rows, tip_units = load(tip, side)
    unmatched |= base_rows.keys() != tip_rows.keys()
    result["sides"][side] = {
        "rows": len(base_rows),
        "unmatched": sorted(f"{f}::{n}" for f, n in base_rows.keys() ^ tip_rows.keys()),
        "rows_differing": [
            {"file": f, "function": n, "base": base_rows[(f, n)][0], "tip": tip_rows[(f, n)][0],
             "base_diagnostics": base_rows[(f, n)][1], "tip_diagnostics": tip_rows[(f, n)][1]}
            for (f, n) in sorted(base_rows.keys() & tip_rows.keys())
            if base_rows[(f, n)] != tip_rows[(f, n)]
        ],
        "units": {"base": base_units, "tip": tip_units},
    }
out.write_text(json.dumps(result, indent=1) + "\n")
print(json.dumps(result, indent=1))
sys.exit(1 if unmatched else 0)
