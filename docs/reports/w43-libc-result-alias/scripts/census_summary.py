#!/usr/bin/env python3
"""Summarise the declared-ABI censuses: per name, outcome counts old vs new,
and every (unit, name) row whose outcome moved between the two readers."""
import collections
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
CENSUS = ROOT / "measure/census"


def rows(path):
    table = {}
    for line in path.read_text().splitlines():
        parts = line.split("\t")
        if len(parts) == 3:
            table[(pathlib.Path(parts[0]).name, parts[1])] = parts[2]
    return table


def per_name(table):
    counts = collections.defaultdict(collections.Counter)
    for (_, name), outcome in table.items():
        counts[name][outcome] += 1
    return {name: dict(sorted(c.items())) for name, c in sorted(counts.items())}


summary = {}
for label in ("manifest", "new13", "restrict14"):
    new = rows(CENSUS / f"{label}-new.txt")
    old_path = CENSUS / f"{label}-old.txt"
    old = rows(old_path)
    moved = collections.Counter()
    examples = {}
    for key in sorted(set(old) | set(new)):
        a, b = old.get(key, "absent"), new.get(key, "absent")
        if a != b:
            moved[(key[1], a, b)] += 1
            examples.setdefault((key[1], a, b), key[0])
    summary[label] = {
        "units_rows_new": len(new),
        "units_rows_old": len(old),
        "old_tool_output_head": old_path.read_text().splitlines()[:2] if not old else None,
        "per_name_new": per_name(new),
        "per_name_old": per_name(old),
        "moved": [
            {"name": n, "old": a, "new": b, "rows": c, "example_unit": examples[(n, a, b)]}
            for (n, a, b), c in sorted(moved.items())
        ],
    }
out = ROOT / "measure/census/summary.json"
out.write_text(json.dumps(summary, indent=1) + "\n")
for label, entry in summary.items():
    print("==", label, "rows old/new", entry["units_rows_old"], entry["units_rows_new"])
    if entry["old_tool_output_head"]:
        print("  old tool:", entry["old_tool_output_head"])
    for move in entry["moved"]:
        print("  moved", move)
    if label != "manifest":
        for name, counts in entry["per_name_new"].items():
            print("  new", name, counts, "| old", entry["per_name_old"].get(name))
