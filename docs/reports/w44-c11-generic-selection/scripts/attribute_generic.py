#!/usr/bin/env python3
"""Attribute every leaf delta between a BEFORE and an AFTER corpus run to the
generic-selection lowering.

Usage: attribute_generic.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS CENSUS_JSON OUT_JSON

CENSUS_JSON is `generic_census.py` output over the captured `-E -P` units: one
row per `_Generic` occurrence with its unit, function, the reader's verdict and
the first line of the statement it sits in (`leaf_line`).

A leaf is `(line, category, description)` within `(unit, function)`; leaves
are compared as multisets. Every differing leaf is classified:

* `generic_leaf_removed` — a removed `call to uncontracted function
  `_Generic`` leaf;
* `at_selection_statement` — any other removed or added leaf at the first line
  of a statement holding an admitted selection (the lowered call's own answer
  replacing the selection's);
* `elsewhere_in_selection_function` — a removed or added leaf elsewhere in a
  function holding an admitted selection (a binding the refused statement used
  to poison, a derived result now tracked);
* `newly_safe_callee` — a removed `call to uncontracted function `F`` leaf
  where `F` is defined in the same unit and moved to `Safe` (W4 now derives
  and validates its summary), in a function that holds no admitted selection;
* `unattributed` — anything else in a function that holds no admitted
  selection.

Function and unit status moves and the diagnostic totals are reported beside
the leaves. Exit 1 if anything is unattributed.
"""

import collections
import gzip
import json
import pathlib
import sys

GENERIC_LEAF = "call to uncontracted function `_Generic`"


def load(path):
    raw = pathlib.Path(path).read_bytes()
    if path.endswith(".gz"):
        raw = gzip.decompress(raw)
    return json.loads(raw)


def index(document):
    functions, units, totals = {}, {}, collections.Counter()
    for unit in document["units"]:
        units[unit["file"]] = unit["status"]
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            leaves = collections.Counter(
                (leaf["line"], leaf["category"], leaf["description"])
                for leaf in function["diagnostics"]
            )
            functions[key] = (function["status"], leaves)
            for leaf in function["diagnostics"]:
                totals[leaf["category"]] += 1
    return functions, units, totals


def main():
    before, units_before, totals_before = index(load(sys.argv[1]))
    after, units_after, totals_after = index(load(sys.argv[2]))
    census = json.loads(pathlib.Path(sys.argv[3]).read_text())
    admitted = collections.defaultdict(set)
    for row in census["rows"]:
        if row["reader"] != "admitted" or not row["function"]:
            continue
        for unit in row["units"]:
            admitted[(unit, row["function"])].add(row["leaf_line"] or row["line"])
    newly_safe = {
        key for key in set(before) & set(after)
        if before[key][0] != "Safe" and after[key][0] == "Safe"
    }
    classes = collections.Counter()
    rows = []
    status_moves = []
    for key in sorted(set(before) | set(after)):
        status_b, leaves_b = before.get(key, (None, collections.Counter()))
        status_a, leaves_a = after.get(key, (None, collections.Counter()))
        removed = leaves_b - leaves_a
        added = leaves_a - leaves_b
        if status_b != status_a:
            status_moves.append(
                {"unit": key[0], "function": key[1], "before": status_b, "after": status_a,
                 "holds_admitted_selection": key in admitted}
            )
        if not removed and not added:
            continue
        for direction, delta in (("removed", removed), ("added", added)):
            for (line, category, description), count in sorted(delta.items()):
                callee = description[len("call to uncontracted function `"):-1] if description.startswith("call to uncontracted function `") else None
                if key not in admitted and direction == "removed" and (key[0], callee) in newly_safe:
                    label = "newly_safe_callee"
                elif key not in admitted:
                    label = "unattributed"
                elif direction == "removed" and description == GENERIC_LEAF:
                    label = "generic_leaf_removed"
                elif line in admitted[key]:
                    label = "at_selection_statement"
                else:
                    label = "elsewhere_in_selection_function"
                classes[(label, direction, category)] += count
                rows.append({
                    "unit": key[0], "function": key[1], "direction": direction,
                    "line": line, "category": category, "description": description,
                    "count": count, "class": label,
                })
    unit_moves = [
        {"unit": unit, "before": units_before.get(unit), "after": units_after.get(unit)}
        for unit in sorted(set(units_before) | set(units_after))
        if units_before.get(unit) != units_after.get(unit)
    ]
    def status_counts(functions):
        return collections.Counter(status for status, _ in functions.values())
    result = {
        "totals_before": dict(totals_before),
        "totals_after": dict(totals_after),
        "function_status_before": dict(status_counts(before)),
        "function_status_after": dict(status_counts(after)),
        "classes": {f"{label}/{direction}/{category}": count
                    for (label, direction, category), count in sorted(classes.items())},
        "status_moves": status_moves,
        "unit_moves": unit_moves,
        "functions_with_admitted_selection": len(admitted),
        "rows": rows,
    }
    pathlib.Path(sys.argv[4]).write_text(json.dumps(result, indent=1) + "\n")
    summary = {k: v for k, v in result.items() if k != "rows"}
    print(json.dumps(summary, indent=1))
    unattributed = sum(count for (label, _, _), count in classes.items() if label == "unattributed")
    print("unattributed", unattributed)
    return 1 if unattributed else 0


if __name__ == "__main__":
    sys.exit(main())
