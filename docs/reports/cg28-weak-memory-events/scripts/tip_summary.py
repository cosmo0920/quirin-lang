#!/usr/bin/env python3
"""CG28-WEAK-MEMORY-EVENTS: summarise the rebased tip against the card's base
runs (corpus base AFTER -> tip; system and slice base BEFORE -> tip).

usage: tip_summary.py   (run from this directory; writes ../artifacts/after-base-to-tip.json)
"""
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
PRIVATE = HERE.parents[3] / "private/cg28"
SLICES = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
PAIRS = {
    "corpus (base AFTER -> tip)": (PRIVATE / "after", PRIVATE / "tip"),
    "system (base BEFORE -> tip)": (PRIVATE / "before-system", PRIVATE / "tip-system"),
    "slice (base BEFORE -> tip)": (SLICES / "cg28-before", SLICES / "cg28-tip"),
}
record = {}
for name, (before, after) in PAIRS.items():
    out = subprocess.run([sys.executable, str(HERE / "compare.py"), str(before), str(after)],
                         check=True, capture_output=True, text=True).stdout
    data = json.loads(out)
    record[name] = {
        "totals": data["totals"],
        "move_counts": data["move_counts"],
        "moves": [f"{m['unit']}::{m['function']} {m['before']} -> {m['after']}" for m in data["moves"]],
        "same_status_functions_with_changed_diagnostics": data["same_status_functions_with_changed_diagnostics"],
        "leaf_delta": data["leaf_delta"],
    }
(HERE.parent / "artifacts/after-base-to-tip.json").write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
print(json.dumps(record, indent=1, ensure_ascii=False))
