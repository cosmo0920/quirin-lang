#!/usr/bin/env python3
"""moved.py BASE.json TIP.json -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4:
every row whose status differs between two opt-in `current` reports (rows
keyed by unit file, function and occurrence), with its base leaves and the
tip's contract dependencies (declarations and summaries the Safe row rests
on), as JSON lines; then the status-move totals."""
import collections
import json
import sys


def rows(path):
    data = json.load(open(path))
    out = {}
    for unit in data["units"]:
        seen = collections.Counter()
        for function in unit["functions"]:
            seen[function["function"]] += 1
            out[(unit["file"], function["function"], seen[function["function"]])] = function
    return out


def main() -> int:
    base, tip = rows(sys.argv[1]), rows(sys.argv[2])
    moves = collections.Counter()
    for key in sorted(set(base) | set(tip)):
        before, after = base.get(key), tip.get(key)
        a = before["status"] if before else "absent"
        b = after["status"] if after else "absent"
        if a == b:
            continue
        moves[f"{a} -> {b}"] += 1
        deps = (after or {}).get("contract_dependencies") or {}
        print(json.dumps({
            "file": key[0], "function": key[1], "occurrence": key[2], "move": f"{a} -> {b}",
            "base_leaves": [f"{d.get('line')}: {d['description']}" for d in (before or {}).get("diagnostics", [])],
            "tip_declarations": deps.get("declarations", []),
            "tip_summaries": deps.get("summaries", []),
        }, sort_keys=True))
    print(json.dumps({"moves": dict(moves)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
