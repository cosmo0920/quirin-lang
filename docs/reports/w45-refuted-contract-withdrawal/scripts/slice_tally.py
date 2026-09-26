#!/usr/bin/env python3
"""Tally a `quirin-audit current` slice report (card W45-REFUTED-CONTRACT-WITHDRAWAL).

Usage: slice_tally.py REPORT [--before REPORT] [--functions]

Units by status; distinct functions (by file and name, the worst status over
the static and shared variants, `not projected` counted apart); diagnostic
categories. With `--before`, every distinct function whose status or leaf
multiset differs, with the leaves added and removed.
"""
import collections
import gzip
import json
import sys

ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def distinct(report):
    rows = collections.defaultdict(list)
    projected = set()
    for unit in report["units"]:
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            projected.add(key)
            rows[key].append(function)
    not_projected = set()
    for unit in report["units"]:
        for name in unit.get("not_projected", []):
            key = (unit["file"], name if isinstance(name, str) else json.dumps(name))
            if key not in projected:
                not_projected.add(key)
    return rows, not_projected


def worst(functions):
    return max((f["status"] for f in functions), key=lambda s: ORDER[s])


def leaves(functions):
    counter = collections.Counter()
    for function in functions:
        for diagnostic in function["diagnostics"]:
            counter[(diagnostic["category"], diagnostic["description"], diagnostic.get("origin"))] += 1
    return counter


def tally(report):
    rows, not_projected = distinct(report)
    categories = collections.Counter()
    for unit in report["units"]:
        for function in unit["functions"]:
            for diagnostic in function["diagnostics"]:
                categories[diagnostic["category"]] += 1
    functions = collections.Counter(worst(f) for f in rows.values())
    return {
        "units": dict(collections.Counter(u["status"] for u in report["units"])),
        "functions": dict(functions),
        "not_projected": len(not_projected),
        "categories": dict(categories),
    }


def main():
    after = load(sys.argv[1])
    result = {"after": tally(after)}
    if "--before" in sys.argv:
        before = load(sys.argv[sys.argv.index("--before") + 1])
        result["before"] = tally(before)
        brows, _ = distinct(before)
        arows, _ = distinct(after)
        moves = []
        for key in sorted(set(brows) | set(arows)):
            b = brows.get(key, [])
            a = arows.get(key, [])
            bl, al = leaves(b), leaves(a)
            bs = worst(b) if b else None
            as_ = worst(a) if a else None
            if bs != as_ or bl != al:
                moves.append({
                    "file": key[0], "function": key[1], "before": bs, "after": as_,
                    "removed": [list(k) + [v] for k, v in sorted((bl - al).items())],
                    "added": [list(k) + [v] for k, v in sorted((al - bl).items())],
                })
        result["moves"] = moves
    json.dump(result, sys.stdout, indent=1)
    print()


if __name__ == "__main__":
    main()
