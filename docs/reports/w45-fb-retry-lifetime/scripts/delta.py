#!/usr/bin/env python3
"""Card W45-FB-RETRY-LIFETIME (copied from W45-PARENTHESISED-CALLEE): the BEFORE/AFTER delta of two corpus runs.

Usage: delta.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS [--json OUT]

Reads two `resource-diagnostics.json` files (plain or gzip) and prints:

* the function and unit status tallies on each side;
* the diagnostic category totals on each side;
* every function whose status moved, with its removed and added leaves;
* every removed or added leaf (a (unit, function, line, category,
  description) multiset element), grouped by function.

`--json OUT` writes the same facts as one JSON document. The script only
reads; attribution by reading is the report's.
"""

import collections
import gzip
import json
import pathlib
import sys


def load(path):
    data = pathlib.Path(path).read_bytes()
    if data[:2] == b"\x1f\x8b":
        data = gzip.decompress(data)
    return json.loads(data)


def index(document):
    statuses = {}
    units = {}
    leaves = collections.Counter()
    categories = collections.Counter()
    for unit in document["units"]:
        units[unit["file"]] = unit["status"]
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            statuses[key] = function["status"]
            for diagnostic in function["diagnostics"]:
                categories[diagnostic["category"]] += 1
                leaves[
                    (
                        unit["file"],
                        function["function"],
                        diagnostic["line"],
                        diagnostic["category"],
                        diagnostic.get("description", ""),
                    )
                ] += 1
    return statuses, units, leaves, categories


def main() -> int:
    before = index(load(sys.argv[1]))
    after = index(load(sys.argv[2]))
    out = None
    if "--json" in sys.argv[3:]:
        out = pathlib.Path(sys.argv[sys.argv.index("--json", 3) + 1])
    b_status, b_units, b_leaves, b_categories = before
    a_status, a_units, a_leaves, a_categories = after
    removed = b_leaves - a_leaves
    added = a_leaves - b_leaves
    moves = sorted(
        (key, b_status.get(key), a_status.get(key))
        for key in set(b_status) | set(a_status)
        if b_status.get(key) != a_status.get(key)
    )
    unit_moves = sorted(
        (key, b_units.get(key), a_units.get(key))
        for key in set(b_units) | set(a_units)
        if b_units.get(key) != a_units.get(key)
    )
    by_function = collections.defaultdict(lambda: {"removed": [], "added": []})
    for leaf, count in removed.items():
        by_function[leaf[:2]]["removed"].extend([list(leaf[2:])] * count)
    for leaf, count in added.items():
        by_function[leaf[:2]]["added"].extend([list(leaf[2:])] * count)
    document = {
        "functions": {
            "before": dict(collections.Counter(b_status.values())),
            "after": dict(collections.Counter(a_status.values())),
        },
        "units": {
            "before": dict(collections.Counter(b_units.values())),
            "after": dict(collections.Counter(a_units.values())),
        },
        "categories": {"before": dict(b_categories), "after": dict(a_categories)},
        "removed_leaves": sum(removed.values()),
        "added_leaves": sum(added.values()),
        "function_moves": [
            {"unit": key[0], "function": key[1], "before": old, "after": new}
            for key, old, new in moves
        ],
        "unit_moves": [
            {"unit": key, "before": old, "after": new} for key, old, new in unit_moves
        ],
        "leaves_by_function": [
            {"unit": key[0], "function": key[1], **value}
            for key, value in sorted(by_function.items())
        ],
    }
    print("functions", document["functions"])
    print("units", document["units"])
    print("categories before", sorted(b_categories.items()))
    print("categories after ", sorted(a_categories.items()))
    print("leaves removed", document["removed_leaves"], "added", document["added_leaves"])
    print("function moves", len(moves))
    for key, old, new in moves:
        print(f"  {key[0]} :: {key[1]}: {old} -> {new}")
    print("unit moves", len(unit_moves))
    for key, old, new in unit_moves:
        print(f"  {key}: {old} -> {new}")
    for entry in document["leaves_by_function"]:
        print(f"{entry['unit']} :: {entry['function']}")
        for line, category, description in entry["removed"]:
            print(f"  - L{line} {category}: {description}")
        for line, category, description in entry["added"]:
            print(f"  + L{line} {category}: {description}")
    if out is not None:
        out.write_text(json.dumps(document, indent=1, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
