#!/usr/bin/env python3
"""Card JOINED-PROVENANCE-CYCLES: the leaf-level three-way split.

Leaf identity is `(unit file, function, preprocessed line, category,
description)` — the **internal** coordinate, per constraint 12a; `origin`, the
derived original `file:line`, is carried alongside for reading only and is
never part of the key. Unit and function status moves are reported separately,
because a histogram difference is not a per-entity move.

Usage: python3 measure/joined-provenance-cycles/leaves.py [before] [after]
"""
import collections
import json
import pathlib
import sys

M = pathlib.Path(__file__).resolve().parent


def load(stage):
    return json.loads((M / "runs" / stage / "resource-diagnostics.json").read_text())


def leaves(doc):
    out = {}
    for u in doc["units"]:
        for f in u.get("functions") or []:
            for d in f.get("diagnostics") or []:
                key = (u["file"], f["function"], d["line"], d["category"], d["description"])
                out[key] = d.get("origin")
    return out


def statuses(doc):
    units = {u["file"]: u["status"] for u in doc["units"]}
    funcs = {}
    for u in doc["units"]:
        for f in u.get("functions") or []:
            funcs[(u["file"], f["function"])] = f["status"]
    return units, funcs


def main():
    before_stage = sys.argv[1] if len(sys.argv) > 1 else "before"
    after_stage = sys.argv[2] if len(sys.argv) > 2 else "after"
    b, a = load(before_stage), load(after_stage)
    lb, la = leaves(b), leaves(a)

    print(f"BEFORE = runs/{before_stage}   AFTER = runs/{after_stage}")
    print("\n== diagnostic_counts ==")
    keys = sorted(set(b["diagnostic_counts"]) | set(a["diagnostic_counts"]))
    for k in keys:
        x, y = b["diagnostic_counts"].get(k, 0), a["diagnostic_counts"].get(k, 0)
        mark = "" if x == y else f"   <-- {y - x:+d}"
        print(f"  {k:<16} {x:>7} -> {y:>7}{mark}")

    added, removed = set(la) - set(lb), set(lb) - set(la)
    print("\n== leaf three-way split ==")
    print(f"  added (only in AFTER):   {len(added)}")
    print(f"  removed (only in BEFORE):{len(removed)}")
    print(f"  unmoved:                 {len(set(lb) & set(la))}")

    for label, group in (("REMOVED", removed), ("ADDED", added)):
        if not group:
            continue
        print(f"\n== {label}: by category ==")
        for cat, n in collections.Counter(k[3] for k in group).most_common():
            print(f"  {n:>6}  {cat}")
        print(f"== {label}: by (file, function) ==")
        per = collections.Counter((k[0], k[1]) for k in group)
        for (fl, fn), n in per.most_common(25):
            print(f"  {n:>4}  {fl} :: {fn}")
        if len(per) > 25:
            print(f"  ... and {len(per) - 25} more")
        print(f"== {label}: every leaf ==" if len(group) <= 60 else f"== {label}: first 60 ==")
        src = lb if label == "REMOVED" else la
        for k in sorted(group)[:60]:
            print(f"  {k[0]} :: {k[1]} :: L{k[2]} :: {k[3]} :: {k[4]}  [origin {src[k]}]")

    ub, fb = statuses(b)
    ua, fa = statuses(a)
    umoves = [(k, ub[k], ua[k]) for k in ub if k in ua and ub[k] != ua[k]]
    fmoves = [(k, fb[k], fa[k]) for k in fb if k in fa and fb[k] != fa[k]]
    print(f"\n== unit status moves: {len(umoves)} ==")
    for k, x, y in umoves:
        print(f"  {k}: {x} -> {y}")
    print(f"== function status moves: {len(fmoves)} ==")
    for k, x, y in fmoves:
        print(f"  {k[0]} :: {k[1]}: {x} -> {y}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
