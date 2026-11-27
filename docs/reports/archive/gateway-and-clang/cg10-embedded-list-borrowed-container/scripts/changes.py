#!/usr/bin/env python3
"""changes.py BEFORE AFTER ROWS23.json OUT.json -- every row whose leaves or
status changed between two opt-in `current` reports, with the leaves removed
and added, and the standing of each of the 23 embedded-list sole rows."""
import collections
import json
import sys

GATE = "indirect memory access is outside the resource projection"


def rows(path):
    out = {}
    for unit in json.load(open(path))["units"]:
        seen = collections.Counter()
        for function in unit.get("functions", []):
            seen[function["function"]] += 1
            key = (unit["file"], function["function"], seen[function["function"]])
            out[key] = (
                function["status"],
                collections.Counter((d["description"], d.get("origin")) for d in function.get("diagnostics", [])),
            )
    return out


def main() -> int:
    before, after = rows(sys.argv[1]), rows(sys.argv[2])
    sole = {(r["file"], r["function"]) for r in json.load(open(sys.argv[3]))}
    changed = []
    for key in sorted(set(before) | set(after)):
        b, a = before.get(key), after.get(key)
        if b == a:
            continue
        removed = (b[1] - a[1]) if b and a else None
        added = (a[1] - b[1]) if b and a else None
        changed.append({
            "file": key[0], "function": key[1], "occurrence": key[2],
            "status": [b[0] if b else None, a[0] if a else None],
            "removed": sorted(f"{d} @ {o}" for d, o in (removed or {}).elements()),
            "added": sorted(f"{d} @ {o}" for d, o in (added or {}).elements()),
            "sole_embedded_list_row": (key[0], key[1]) in sole,
            "access_left_after": sum(n for (d, _), n in a[1].items() if d == GATE) if a else None,
        })
    standing = []
    for file, function in sorted(sole):
        a = after.get((file, function, 1))
        standing.append({"file": file, "function": function, "status": a[0],
                         "leaves": sorted({d for d, _ in a[1]})})
    moves = [c for c in changed if c["status"][0] != c["status"][1]]
    json.dump({"moves": moves, "changed_rows": changed, "sole_rows_after": standing},
              open(sys.argv[4], "w"), indent=1)
    print(f"status moves {len(moves)}; rows with changed leaves {len(changed)}; "
          f"sole rows {len(standing)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
