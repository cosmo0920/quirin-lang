#!/usr/bin/env python3
"""Leaf-by-leaf difference between two audit runs' diagnostics.

Usage: diff_runs.py BEFORE_RUN AFTER_RUN OUT_JSON

A leaf is `(unit, function, line, category, description)`. Reports removed
and added leaves, function and unit status moves, the per-category counts and
whether every other top-level field (the `scope` string aside) is identical.
"""

import collections
import json
import pathlib
import sys


def leaves(document):
    rows = collections.Counter()
    statuses = {}
    units = {}
    for unit in document["units"]:
        key = f"{unit['library']}/{unit['file']}"
        units[key] = unit.get("status")
        for function in unit.get("functions", []):
            statuses[(key, function["function"])] = function.get("status")
            for leaf in function.get("diagnostics", []):
                rows[(key, function["function"], leaf["line"], leaf["category"], leaf["description"])] += 1
    return rows, statuses, units


def main() -> int:
    before = json.loads((pathlib.Path(sys.argv[1]) / "resource-diagnostics.json").read_text())
    after = json.loads((pathlib.Path(sys.argv[2]) / "resource-diagnostics.json").read_text())
    b_rows, b_status, b_units = leaves(before)
    a_rows, a_status, a_units = leaves(after)
    removed = sorted((b_rows - a_rows).elements())
    added = sorted((a_rows - b_rows).elements())
    function_moves = sorted(
        (key, b_status.get(key), a_status.get(key))
        for key in set(b_status) | set(a_status)
        if b_status.get(key) != a_status.get(key)
    )
    unit_moves = sorted(
        (key, b_units.get(key), a_units.get(key))
        for key in set(b_units) | set(a_units)
        if b_units.get(key) != a_units.get(key)
    )
    other_fields = sorted(
        key
        for key in set(before) | set(after)
        if key not in ("units", "scope") and before.get(key) != after.get(key)
    )

    def status_counts(statuses):
        return dict(collections.Counter(statuses.values()))

    result = {
        "removed": [list(row) for row in removed],
        "added": [list(row) for row in added],
        "function_moves": [list(row) for row in function_moves],
        "unit_moves": [list(row) for row in unit_moves],
        "diagnostic_counts": {"before": before.get("diagnostic_counts"), "after": after.get("diagnostic_counts")},
        "function_statuses": {"before": status_counts(b_status), "after": status_counts(a_status)},
        "unit_statuses": {"before": status_counts(b_units), "after": status_counts(a_units)},
        "scope_changed": before.get("scope") != after.get("scope"),
        "other_top_level_fields_differing": other_fields,
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({key: (len(value) if isinstance(value, list) else value) for key, value in result.items()}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
