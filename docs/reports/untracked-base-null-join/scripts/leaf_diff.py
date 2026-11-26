#!/usr/bin/env python3
"""leaf_diff.py BASE.json TIP.json -- card UNTRACKED-BASE-NULL-JOIN-DISCHARGE:
for every row whose diagnostics differ between two opt-in `current` reports
(keyed by unit index and function position), the diagnostics added and
removed as multisets. Prints one JSON line per row and a summary line."""
import collections
import json
import sys


def key(diagnostic: dict) -> str:
    return json.dumps(diagnostic, sort_keys=True)


def main() -> int:
    base = json.load(open(sys.argv[1]))
    tip = json.load(open(sys.argv[2]))
    rows = added_total = removed_total = 0
    for index, (before, after) in enumerate(zip(base["units"], tip["units"])):
        for row_b, row_t in zip(before.get("functions", []), after.get("functions", [])):
            if row_b["diagnostics"] == row_t["diagnostics"]:
                continue
            old = collections.Counter(key(d) for d in row_b["diagnostics"])
            new = collections.Counter(key(d) for d in row_t["diagnostics"])
            added = sorted((new - old).elements())
            removed = sorted((old - new).elements())
            rows += 1
            added_total += len(added)
            removed_total += len(removed)
            print(json.dumps({
                "unit": index, "file": before["file"], "function": row_b["function"],
                "status": [row_b["status"], row_t["status"]],
                "added": [json.loads(a) for a in added],
                "removed": [json.loads(r) for r in removed],
            }))
    print(json.dumps({"rows": rows, "added": added_total, "removed": removed_total}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
