#!/usr/bin/env python3
"""diffrows.py BASE.json TIP.json -- every row whose status or leaf multiset
differs, rows keyed by unit index, unit file, function and occurrence (the
fluent-bit-system report carries two units per file, one per
configuration), then the status-move totals and both sides' totals."""
import collections
import json
import sys


def rows(path):
    data = json.load(open(path))
    out = {}
    totals = collections.Counter()
    for index, unit in enumerate(data["units"]):
        seen = collections.Counter()
        for function in unit.get("functions", []):
            seen[function["function"]] += 1
            totals[function["status"]] += 1
            leaves = collections.Counter(
                f"{d.get('line')}: {d['description']}" for d in function["diagnostics"])
            out[(index, unit["file"], function["function"], seen[function["function"]])] = (
                function["status"], leaves)
    return out, totals


def main() -> int:
    (base, bt), (tip, tt) = rows(sys.argv[1]), rows(sys.argv[2])
    moves = collections.Counter()
    for key in sorted(set(base) | set(tip)):
        b = base.get(key, ("absent", collections.Counter()))
        t = tip.get(key, ("absent", collections.Counter()))
        if b == t:
            continue
        if b[0] != t[0]:
            moves[f"{b[0]} -> {t[0]}"] += 1
        print(f"unit {key[0]} {key[1]} {key[2]} #{key[3]}: {b[0]} -> {t[0]}")
        for leaf in sorted((b[1] - t[1]).elements()):
            print("   -", leaf)
        for leaf in sorted((t[1] - b[1]).elements()):
            print("   +", leaf)
    print(json.dumps({"moves": dict(moves), "base": dict(bt), "tip": dict(tt)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
