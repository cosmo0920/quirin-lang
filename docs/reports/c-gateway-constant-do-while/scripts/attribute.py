#!/usr/bin/env python3
"""Card C-GATEWAY-CONSTANT-DO-WHILE: attribute every moved leaf.

A re-pin that reports only headline counts is not attributable. This walks the
BEFORE and AFTER `diagnostics.json` leaf by leaf, keyed on the **internal
preprocessed coordinate** (constraint 12a), and prints:

  * the headline category histogram, before and after;
  * every unit status move and every function status move;
  * every added and removed leaf, with its library, unit, function, internal
    line, presented origin, category and text;
  * the three-way split over affected functions that the card asks for:
    leaf-free / exchanged-for-another-refusal / unmoved.

Writes nothing outside `measure/` unless given an output path.
"""
import collections
import json
import pathlib
import sys

M = pathlib.Path(__file__).resolve().parent


def load(stage):
    path = M / "runs" / stage / "resource-diagnostics.json"
    return json.loads(path.read_text())


def leaves(document):
    """Every diagnostic occurrence, keyed by its internal coordinate."""
    out = {}
    for unit in document["units"]:
        for function in unit["functions"]:
            for diagnostic in function["diagnostics"]:
                key = (
                    unit["library"],
                    unit["file"],
                    function["function"],
                    diagnostic["line"],
                    diagnostic["category"],
                    diagnostic["description"],
                )
                out[key] = out.get(key, 0) + 1
    return out


def statuses(document):
    units = {}
    functions = {}
    for unit in document["units"]:
        units[(unit["library"], unit["file"])] = unit["status"]
        for function in unit["functions"]:
            functions[(unit["library"], unit["file"], function["function"])] = function[
                "status"
            ]
    return units, functions


def histogram(document):
    return dict(sorted(document["diagnostic_counts"].items()))


def main():
    before, after = load("before"), load("after")
    print("== headline ==")
    print("BEFORE", json.dumps(histogram(before)))
    print("AFTER ", json.dumps(histogram(after)))
    moved = {
        k: (histogram(before).get(k, 0), v)
        for k, v in histogram(after).items()
        if histogram(before).get(k, 0) != v
    }
    for k, (b, a) in moved.items():
        print(f"  {k}: {b} -> {a}  ({a - b:+d})")
    if not moved:
        print("  (no category moved)")

    bu, bf = statuses(before)
    au, af = statuses(after)
    print("\n== unit status moves ==")
    unit_moves = [(k, bu[k], au[k]) for k in bu if k in au and bu[k] != au[k]]
    for k, b, a in unit_moves:
        print(f"  {k[0]}/{k[1]}: {b} -> {a}")
    print(f"  total: {len(unit_moves)}")

    print("\n== function status moves ==")
    fn_moves = [(k, bf[k], af[k]) for k in bf if k in af and bf[k] != af[k]]
    for k, b, a in fn_moves:
        print(f"  {k[0]}/{k[1]}::{k[2]}: {b} -> {a}")
    print(f"  total: {len(fn_moves)}")

    bl, al = leaves(before), leaves(after)
    removed = {k: v for k, v in bl.items() if al.get(k, 0) < v}
    added = {k: v for k, v in al.items() if bl.get(k, 0) < v}
    print(f"\n== leaves ==\n  removed: {len(removed)}   added: {len(added)}")

    by_category = collections.Counter()
    for k in removed:
        by_category[("removed", k[4])] += 1
    for k in added:
        by_category[("added", k[4])] += 1
    for (direction, category), n in sorted(by_category.items()):
        print(f"  {direction:>7} {category:<16} {n}")

    def show(title, table, limit):
        print(f"\n-- {title} ({len(table)}) --")
        for k in sorted(table)[:limit]:
            library, unit, function, line, category, text = k
            print(f"  {library}/{unit}::{function}")
            print(f"    internal L{line}  {category}: {text}")
        if len(table) > limit:
            print(f"  … {len(table) - limit} more")

    show("removed leaves", removed, 60)
    show("added leaves", added, 60)

    # The three-way split the card asks for, over every function that had at
    # least one leaf move.
    touched = {k[:3] for k in removed} | {k[:3] for k in added}
    leaf_free, exchanged, unmoved = [], [], []
    for key in sorted(touched):
        before_here = {k: v for k, v in bl.items() if k[:3] == key}
        after_here = {k: v for k, v in al.items() if k[:3] == key}
        if not after_here and before_here:
            leaf_free.append(key)
        elif len(after_here) < len(before_here):
            exchanged.append(key)
        else:
            unmoved.append(key)
    print("\n== three-way split over affected functions ==")
    print(f"  leaf-free (every leaf withdrawn):        {len(leaf_free)}")
    for k in leaf_free:
        print(f"    {k[0]}/{k[1]}::{k[2]}")
    print(f"  fewer leaves (exchanged/withdrawn):      {len(exchanged)}")
    for k in exchanged:
        print(f"    {k[0]}/{k[1]}::{k[2]}  {len(
            [1 for x in bl if x[:3] == k])} -> {len([1 for x in al if x[:3] == k])}")
    print(f"  same or more leaves:                     {len(unmoved)}")
    for k in unmoved:
        print(f"    {k[0]}/{k[1]}::{k[2]}  {len(
            [1 for x in bl if x[:3] == k])} -> {len([1 for x in al if x[:3] == k])}")

    if len(sys.argv) > 1:
        pathlib.Path(sys.argv[1]).write_text(
            json.dumps(
                {
                    "before": histogram(before),
                    "after": histogram(after),
                    "unit_moves": [[list(k), b, a] for k, b, a in unit_moves],
                    "function_moves": [[list(k), b, a] for k, b, a in fn_moves],
                    "removed": [list(k) for k in sorted(removed)],
                    "added": [list(k) for k in sorted(added)],
                    "split": {
                        "leaf_free": [list(k) for k in leaf_free],
                        "fewer_leaves": [list(k) for k in exchanged],
                        "same_or_more": [list(k) for k in unmoved],
                    },
                },
                indent=1,
            )
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
