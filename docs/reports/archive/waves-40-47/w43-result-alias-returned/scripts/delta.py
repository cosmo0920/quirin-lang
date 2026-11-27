#!/usr/bin/env python3
"""Leaf-by-leaf delta between two corpus runs' `resource-diagnostics.json`.

Usage: delta.py BEFORE_DIR AFTER_DIR OUT_JSON

Each diagnostic is a leaf keyed by (unit file, function, category, description,
origin); leaves are compared as multisets. Reports unit and function status
movement, category totals on both sides, and every removed or added leaf. Exits
0 when it wrote the file; it asserts nothing about the delta itself.
"""

import collections
import json
import pathlib
import sys


def load(run):
    data = json.loads((pathlib.Path(run) / "resource-diagnostics.json").read_text())
    units, functions, leaves = {}, {}, collections.Counter()
    for unit in data["units"]:
        name = unit["file"]
        units[name] = unit.get("status")
        for function in unit["functions"]:
            functions[(name, function["function"])] = function["status"]
            for leaf in function["diagnostics"]:
                leaves[(name, function["function"], leaf["category"], leaf["description"], leaf["origin"])] += 1
    return data["diagnostic_counts"], units, functions, leaves


def main():
    before_dir, after_dir, out = sys.argv[1:4]
    b_counts, b_units, b_functions, b_leaves = load(before_dir)
    a_counts, a_units, a_functions, a_leaves = load(after_dir)
    tally = lambda functions: dict(collections.Counter(functions.values()))
    moves = [
        {"unit": key[0], "function": key[1], "before": b_functions.get(key), "after": a_functions.get(key)}
        for key in sorted(set(b_functions) | set(a_functions))
        if b_functions.get(key) != a_functions.get(key)
    ]
    unit_moves = [
        {"unit": key, "before": b_units.get(key), "after": a_units.get(key)}
        for key in sorted(set(b_units) | set(a_units))
        if b_units.get(key) != a_units.get(key)
    ]
    removed = b_leaves - a_leaves
    added = a_leaves - b_leaves
    as_rows = lambda counter: [
        {"unit": k[0], "function": k[1], "category": k[2], "description": k[3], "origin": k[4], "count": n}
        for k, n in sorted(counter.items())
    ]
    result = {
        "before": before_dir,
        "after": after_dir,
        "diagnostic_counts": {"before": b_counts, "after": a_counts},
        "function_status": {"before": tally(b_functions), "after": tally(a_functions)},
        "unit_status": {"before": tally(b_units), "after": tally(a_units)},
        "function_moves": moves,
        "unit_moves": unit_moves,
        "removed_leaves": as_rows(removed),
        "added_leaves": as_rows(added),
        "removed_total": sum(removed.values()),
        "added_total": sum(added.values()),
    }
    pathlib.Path(out).write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps({k: result[k] for k in (
        "diagnostic_counts", "function_status", "unit_status", "function_moves",
        "unit_moves", "removed_total", "added_total")}, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
