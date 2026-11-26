#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card PRIORITY0-ATTRIBUTION: the NULL-dereference property's moves between
the measurement's sides (read-only).

    python3 null_moves.py MEASUREMENT_DIR OUT.json

Reads each side's `<route>-null.jsonl` (property `c.null.deref.v1`), keys a
row by library, file, function and its occurrence, and lists every row whose
`null.verdict` (or `combined`) differs between prev, common and tip, with the
checks whose outcome changed.
"""

import collections
import json
import sys
from pathlib import Path

SUB = {"corpus": "corpus", "twelve-home": "twelve-home", "system": "system", "slice": "slice"}


def load(path):
    rows = {}
    seen = collections.Counter()
    for line in path.read_text().splitlines():
        if not line.strip():
            continue
        r = json.loads(line)
        base = (r.get("library"), r.get("file"), r.get("function"))
        seen[base] += 1
        rows[base + (seen[base],)] = r
    return rows


def verdict(r):
    n = r.get("null") or {}
    return n.get("verdict") or n.get("status")


def main():
    measurement = Path(sys.argv[1])
    out = {}
    for route in SUB:
        sides = {}
        for side in ("prev", "common", "tip"):
            path = measurement / "sides" / side / route / f"{route}-null.jsonl"
            if path.exists():
                sides[side] = load(path)
        if len(sides) < 3:
            continue
        moved = []
        for key in sorted(set().union(*sides.values()), key=str):
            v = [verdict(sides[s][key]) if key in sides[s] else None for s in ("prev", "common", "tip")]
            c = [sides[s][key].get("combined") if key in sides[s] else None for s in ("prev", "common", "tip")]
            if len(set(v)) > 1:
                checks = {}
                for s in ("prev", "common", "tip"):
                    if key in sides[s]:
                        checks[s] = sorted((ch.get("origin"), ch.get("kind"), ch.get("outcome"))
                                           for ch in (sides[s][key].get("null") or {}).get("checks", []))
                changed = sorted(set(map(tuple, checks.get("common", []))) ^ set(map(tuple, checks.get("tip", []))))
                changed1 = sorted(set(map(tuple, checks.get("prev", []))) ^ set(map(tuple, checks.get("common", []))))
                moved.append({"row": list(key), "null": v, "combined": c,
                              "checks_changed_prev_common": changed1, "checks_changed_common_tip": changed})
        out[route] = moved
        print(route, len(moved), [(m["row"][2], m["null"]) for m in moved])
    Path(sys.argv[2]).write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
