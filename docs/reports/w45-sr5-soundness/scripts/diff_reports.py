#!/usr/bin/env python3
"""Leaf-level and status-level difference between two resource-diagnostics
reports (corpus arbiter or slice).

usage: diff_reports.py BEFORE AFTER OUT.json

Keys a function by (unit file, variant index, function); a leaf by
(category, description, origin). Records every removed and added leaf, every
status move, the unit and function tallies, and the category counts.
"""
import collections
import gzip
import json
import sys


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def index(report):
    functions = {}
    units = {}
    seen = collections.Counter()
    for unit in report["units"]:
        variant = seen[unit["file"]]
        seen[unit["file"]] += 1
        units[(unit["file"], variant)] = unit.get("status")
        for function in unit["functions"]:
            leaves = collections.Counter(
                (leaf["category"], leaf["description"], leaf["origin"]) for leaf in function["diagnostics"])
            functions[(unit["file"], variant, function["function"])] = (function["status"], leaves)
    return units, functions


def tally(units, functions):
    unit_status = collections.Counter(units.values())
    function_status = collections.Counter(status for status, _ in functions.values())
    distinct = {}
    for (file, _, name), (status, _) in functions.items():
        rank = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}.get(status, 2)
        key = (file, name)
        distinct[key] = max(distinct.get(key, -1), rank)
    categories = collections.Counter()
    for _, leaves in functions.values():
        for (category, _, _), count in leaves.items():
            categories[category] += count
    return {
        "units": dict(unit_status),
        "function_rows": dict(function_status),
        "distinct_functions_worst": dict(collections.Counter(
            ["Safe", "Pending", "Unknown", "Unsafe"][rank] for rank in distinct.values())),
        "categories": dict(categories),
    }


def main():
    before_units, before = index(load(sys.argv[1]))
    after_units, after = index(load(sys.argv[2]))
    removed, added, moves = [], [], []
    for key in sorted(set(before) | set(after)):
        b_status, b_leaves = before.get(key, ("absent", collections.Counter()))
        a_status, a_leaves = after.get(key, ("absent", collections.Counter()))
        if b_status != a_status:
            moves.append({"function": list(key), "before": b_status, "after": a_status})
        for leaf, count in (b_leaves - a_leaves).items():
            removed.append({"function": list(key), "leaf": list(leaf), "count": count})
        for leaf, count in (a_leaves - b_leaves).items():
            added.append({"function": list(key), "leaf": list(leaf), "count": count})
    unit_moves = [
        {"unit": list(key), "before": before_units.get(key), "after": after_units.get(key)}
        for key in sorted(set(before_units) | set(after_units))
        if before_units.get(key) != after_units.get(key)
    ]
    out = {
        "before": tally(before_units, before),
        "after": tally(after_units, after),
        "removed_leaves": sum(item["count"] for item in removed),
        "added_leaves": sum(item["count"] for item in added),
        "moving_functions": len({tuple(item["function"]) for item in removed + added} | {tuple(m["function"]) for m in moves}),
        "status_moves": moves,
        "unit_moves": unit_moves,
        "removed": removed,
        "added": added,
    }
    json.dump(out, open(sys.argv[3], "w"), indent=1)
    print(json.dumps({k: out[k] for k in ("before", "after", "removed_leaves", "added_leaves",
                                          "moving_functions", "status_moves", "unit_moves")}, indent=1))


main()
