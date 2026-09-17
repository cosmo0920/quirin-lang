#!/usr/bin/env python3
"""Card C-GATEWAY-RETURN-CALL-W4: attribute the corpus delta leaf by leaf.

Usage: python3 measure/c-gateway-return-call-w4/census.py

A leaf's identity is `(unit file, function, preprocessed line, category,
description)` — the preprocessed line, per constraint 12a, is the internal
coordinate every pinned comparison is keyed on; `origin` is carried alongside
for reading.
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
    for unit in doc["units"]:
        for fn in unit.get("functions") or []:
            for d in fn.get("diagnostics") or []:
                key = (unit["file"], fn["function"], d["line"], d["category"], d["description"])
                out[key] = d.get("origin")
    return out


def statuses(doc):
    units, fns = {}, {}
    for unit in doc["units"]:
        units[unit["file"]] = unit["status"]
        for fn in unit.get("functions") or []:
            fns[(unit["file"], fn["function"])] = fn["status"]
    return units, fns


def main():
    before, after = load("before"), load("after")
    print("== headline counts ==")
    keys = sorted(set(before["diagnostic_counts"]) | set(after["diagnostic_counts"]))
    for k in keys:
        b = before["diagnostic_counts"].get(k, 0)
        a = after["diagnostic_counts"].get(k, 0)
        mark = "" if a == b else f"   <== {a - b:+d}"
        print(f"  {k:<16} {b:>7} -> {a:>7}{mark}")

    lb, la = leaves(before), leaves(after)
    withdrawn = sorted(set(lb) - set(la))
    added = sorted(set(la) - set(lb))
    print(f"\n== leaves ==\n  withdrawn {len(withdrawn)}\n  added     {len(added)}")

    print("\n-- withdrawn, by description --")
    for desc, n in collections.Counter(k[4] for k in withdrawn).most_common():
        print(f"  {n:>5}  {desc}")
    print("\n-- added, by description --")
    for desc, n in collections.Counter(k[4] for k in added).most_common():
        print(f"  {n:>5}  {desc}")

    ub, fb = statuses(before)
    ua, fa = statuses(after)
    umoved = [(f, ub[f], ua[f]) for f in ub if ub.get(f) != ua.get(f)]
    fmoved = [(k, fb[k], fa[k]) for k in fb if fb.get(k) != fa.get(k)]
    print(f"\n== status moves ==\n  units     {len(umoved)}")
    for f, b, a in umoved:
        print(f"    {f}: {b} -> {a}")
    print(f"  functions {len(fmoved)}")
    for (unit, fn), b, a in sorted(fmoved):
        print(f"    {b:>7} -> {a:<7} {fn}   [{unit}]")

    tally = collections.Counter(a for _, _, a in fmoved)
    print("  function moves by destination:", dict(tally))

    # Every function that lost its last leaf.
    per_fn_before = collections.Counter((k[0], k[1]) for k in lb)
    per_fn_after = collections.Counter((k[0], k[1]) for k in la)
    leaf_free = [k for k in per_fn_before if per_fn_before[k] > 0 and per_fn_after.get(k, 0) == 0]
    print(f"\n== functions that ended leaf-free (had leaves before) ==  {len(leaf_free)}")

    # Bucket (2): functions that changed leaves but did not end leaf-free.
    exchanged = [
        k
        for k in per_fn_before
        if per_fn_after.get(k, 0) > 0
        and any(x[:2] == k for x in withdrawn)
        and any(x[:2] == k for x in added)
    ]
    print(f"== functions that exchanged one refusal for another ==  {len(exchanged)}")
    for k in sorted(exchanged)[:40]:
        print(f"    {k[1]}   [{k[0]}]")

    print("\n-- every withdrawn leaf (unit, function, preprocessed line, origin) --")
    for k in withdrawn:
        print(f"  {k[0]} :: {k[1]} :: L{k[2]} :: {lb[k]} :: {k[4]}")
    if added:
        print("\n-- every added leaf --")
        for k in added:
            print(f"  {k[0]} :: {k[1]} :: L{k[2]} :: {la[k]} :: {k[4]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
