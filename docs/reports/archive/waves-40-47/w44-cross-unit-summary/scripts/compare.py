#!/usr/bin/env python3
"""Compare two W44 resource reports by selected unit and function."""

import collections
import json
import pathlib
import sys


def rows(path):
    document = json.loads(pathlib.Path(path).read_text())
    result = {}
    occurrences = collections.Counter()
    for unit in document["units"]:
        for function in unit.get("functions", []):
            name = (unit.get("library"), unit.get("file"), function["function"])
            # The system/slice selections repeat some source paths for a
            # second compile task. The imported-home trust context changes
            # compile_configuration, so align those repeated rows by their
            # stable selected-command order instead of that changing digest.
            key = name + (occurrences[name],)
            occurrences[name] += 1
            result[key] = function
    return result


def leaves(function):
    return collections.Counter(
        (diagnostic["category"], diagnostic["description"])
        for diagnostic in function.get("diagnostics", [])
    )


def main():
    before, after = map(rows, sys.argv[1:3])
    if before.keys() != after.keys():
        print("denominator changed")
        print("missing", sorted(before.keys() - after.keys())[:10])
        print("added", sorted(after.keys() - before.keys())[:10])
        return 2
    transitions = collections.Counter()
    changed = []
    lost = collections.Counter()
    gained = collections.Counter()
    for key in before:
        old, new = before[key], after[key]
        transitions[(old["status"], new["status"])] += 1
        removed = leaves(old) - leaves(new)
        added = leaves(new) - leaves(old)
        lost.update(removed)
        gained.update(added)
        if old["status"] != new["status"] or removed or added:
            changed.append({
                "key": key,
                "before": old["status"],
                "after": new["status"],
                "removed_leaves": sorted((category, text, count) for (category, text), count in removed.items()),
                "added_leaves": sorted((category, text, count) for (category, text), count in added.items()),
            })
    print(json.dumps({
        "before": sys.argv[1],
        "after": sys.argv[2],
        "functions": len(before),
        "transitions": [
            {"before": old, "after": new, "count": count}
            for (old, new), count in sorted(transitions.items())
        ],
        "removed_leaves": sorted((category, text, count) for (category, text), count in lost.items()),
        "added_leaves": sorted((category, text, count) for (category, text), count in gained.items()),
        "changed": changed,
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
