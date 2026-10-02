#!/usr/bin/env python3
"""W48-BOTTOM-UP-CALLEE-BODIES: every function whose row changed, with its
leaf delta.

Usage: attribute.py BEFORE AFTER OUT_JSON

Rows are merged per (unit, function) as closure_r3.py merges them (the
system and slice routes carry one row per configuration). A leaf is
(category, description, line); the delta is AFTER minus BEFORE per
(category, description), with the lines of added and removed leaves.
"""
import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import closure_r3  # noqa: E402


def rows(path):
    merged = {}
    for unit in closure_r3.load(path)["units"]:
        for entry in unit["functions"]:
            key = (unit["file"], entry["function"])
            record = merged.setdefault(key, {"status": entry["status"], "leaves": collections.Counter()})
            if closure_r3.ORDER.get(entry["status"], 2) > closure_r3.ORDER.get(record["status"], 2):
                record["status"] = entry["status"]
            for leaf in entry.get("diagnostics") or []:
                record["leaves"][(leaf["category"], leaf.get("description") or "", leaf.get("line"))] += 1
    return merged


def main() -> int:
    before, after = rows(sys.argv[1]), rows(sys.argv[2])
    changed = []
    for key in sorted(set(before) | set(after)):
        b = before.get(key, {"status": "absent", "leaves": collections.Counter()})
        a = after.get(key, {"status": "absent", "leaves": collections.Counter()})
        if b == a:
            continue
        added = a["leaves"] - b["leaves"]
        removed = b["leaves"] - a["leaves"]
        changed.append({
            "unit": key[0], "function": key[1], "before": b["status"], "after": a["status"],
            "added": [{"category": c, "description": d, "line": l, "count": n}
                      for (c, d, l), n in sorted(added.items(), key=str)],
            "removed": [{"category": c, "description": d, "line": l, "count": n}
                        for (c, d, l), n in sorted(removed.items(), key=str)]})
    pathlib.Path(sys.argv[3]).write_text(json.dumps(changed, indent=1) + "\n")
    moves = collections.Counter(f"{c['before']} -> {c['after']}" for c in changed if c["before"] != c["after"])
    print(f"changed rows {len(changed)}; moves {dict(moves)}")
    for c in changed:
        if c["before"] != c["after"] or c["added"]:
            print(f"{c['unit']}::{c['function']} {c['before']} -> {c['after']}")
            for leaf in c["added"]:
                print(f"   + {leaf['line']} {leaf['category']}: {leaf['description']}")
            for leaf in c["removed"][:6]:
                print(f"   - {leaf['line']} {leaf['category']}: {leaf['description']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
