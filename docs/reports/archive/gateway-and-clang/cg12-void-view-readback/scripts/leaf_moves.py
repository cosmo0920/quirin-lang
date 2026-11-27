#!/usr/bin/env python3
"""CG12-VOID-VIEW-READBACK: every leaf added or removed between two runs,
per function, with its origin, and the totals by class.

usage: leaf_moves.py BEFORE_RUN AFTER_RUN [--json OUT]
"""

import collections
import json
import pathlib
import sys


def load(run):
    data = json.loads((pathlib.Path(run) / "resource-diagnostics.json").read_bytes())
    out = {}
    for unit in data["units"]:
        for function in unit["functions"]:
            leaves = collections.Counter(
                (leaf.get("category"), (leaf.get("description") or "").split("`")[0].strip(),
                 leaf.get("origin"))
                for leaf in function.get("diagnostics", []))
            out[(unit["file"], function["function"])] = (function["status"], leaves)
    return out


def main() -> int:
    before, after = load(sys.argv[1]), load(sys.argv[2])
    rows = []
    added_total = collections.Counter()
    removed_total = collections.Counter()
    functions_with_added = collections.Counter()
    for key in sorted(after):
        old_status, old = before.get(key, ("absent", collections.Counter()))
        new_status, new = after[key]
        added, removed = new - old, old - new
        if not added and not removed and old_status == new_status:
            continue
        for (category, description, _), count in added.items():
            added_total[(category, description)] += count
        for (category, description, _), count in removed.items():
            removed_total[(category, description)] += count
        for category in {category for (category, _, _) in added}:
            functions_with_added[category] += 1
        rows.append({
            "unit": key[0], "function": key[1], "before": old_status, "after": new_status,
            "added": sorted(f"{n}x {c}: {d} @ {o}" for (c, d, o), n in added.items()),
            "removed": sorted(f"{n}x {c}: {d} @ {o}" for (c, d, o), n in removed.items()),
        })
    record = {
        "functions": len(rows),
        "status_moves": dict(collections.Counter(f"{r['before']} -> {r['after']}" for r in rows)),
        "added": {f"{c}: {d}": n for (c, d), n in added_total.items()},
        "removed": {f"{c}: {d}": n for (c, d), n in removed_total.items()},
        "functions_with_added_leaves_by_category": dict(functions_with_added),
        "rows": rows,
    }
    print(json.dumps({k: v for k, v in record.items() if k != "rows"}, indent=1))
    if len(sys.argv) > 4 and sys.argv[3] == "--json":
        pathlib.Path(sys.argv[4]).write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
