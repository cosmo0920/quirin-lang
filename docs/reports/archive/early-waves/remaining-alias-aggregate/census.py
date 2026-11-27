#!/usr/bin/env python3
"""Exhaustive before/after census of the 185-unit artifacts for card
ALIAS-AGGREGATE.

Card NEIGHBOURS' census, reused byte-for-byte apart from this header and its
example paths, which was itself card PROD-CONTRACTS' and card O04-TRACE's
census extended with a LEAF-LEVEL diff: for every unit whose
diagnostics moved, each individual (function, line, category, description) leaf
that appeared or disappeared is printed, so no movement can be summarised away
into a count. Nothing here edits an artifact.

Read-only. It opens two freshly produced report directories and prints every
movement it finds, in the three directions the card must account for:

* **unit verdicts** — the headline. A newly surfaced provisional `Unsafe` unit
  must be named prominently, so this script prints the whole status histogram
  and every unit whose `status` or `reason_code` moved, in both directions.
* **function verdicts** — every projected function is compared pairwise.
* **diagnostics** — per-unit counts by category, plus a per-description
  inventory, so every movement can be attributed to a rule rather than argued.

    python3 -B docs/reports/remaining-alias-aggregate/census.py \\
        --before measure/runs/before \\
        --after  measure/runs/after
"""

from __future__ import annotations

import argparse
import collections
import json
import pathlib
import sys

DIAGNOSTICS = "resource-diagnostics.json"


def load(directory: pathlib.Path) -> dict:
    return json.loads((directory / DIAGNOSTICS).read_text())


def unit_key(unit: dict) -> str:
    return f"{unit.get('library')}/{unit.get('file')}"


def descriptions(unit: dict) -> collections.Counter:
    counter: collections.Counter = collections.Counter()
    for function in unit.get("functions", []):
        for diagnostic in function.get("diagnostics", []):
            counter[(diagnostic.get("category"), diagnostic.get("description"))] += 1
    return counter


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", required=True, type=pathlib.Path)
    parser.add_argument("--after", required=True, type=pathlib.Path)
    arguments = parser.parse_args()

    before = load(arguments.before)
    after = load(arguments.after)

    units_before = {unit_key(unit): unit for unit in before["units"]}
    units_after = {unit_key(unit): unit for unit in after["units"]}

    print(f"units: {len(units_before)} -> {len(units_after)}")
    added = sorted(set(units_after) - set(units_before))
    removed = sorted(set(units_before) - set(units_after))
    print(f"units added: {added}")
    print(f"units removed: {removed}")

    def histogram(units: dict) -> collections.Counter:
        return collections.Counter(unit.get("status") for unit in units.values())

    print(f"unit status before: {dict(sorted(histogram(units_before).items()))}")
    print(f"unit status after : {dict(sorted(histogram(units_after).items()))}")

    status_moves = []
    reason_moves = []
    for key in sorted(set(units_before) & set(units_after)):
        one, two = units_before[key], units_after[key]
        if one.get("status") != two.get("status"):
            status_moves.append((key, one.get("status"), two.get("status")))
        if one.get("reason_code") != two.get("reason_code"):
            reason_moves.append((key, one.get("reason_code"), two.get("reason_code")))
    print(f"unit status moves: {len(status_moves)}")
    for move in status_moves:
        print(f"  UNIT STATUS MOVE {move[0]}: {move[1]} -> {move[2]}")
    surfaced = [move for move in status_moves if move[2] == "unsafe"]
    print(f"NEWLY SURFACED PROVISIONAL UNSAFE UNITS: {len(surfaced)}")
    for move in surfaced:
        print(f"  *** {move[0]}: {move[1]} -> {move[2]}")
    print(f"unit reason_code moves: {len(reason_moves)}")
    for move in reason_moves:
        print(f"  REASON {move[0]}: {move[1]} -> {move[2]}")

    # Functions.
    def functions(units: dict) -> dict:
        out = {}
        for key, unit in units.items():
            for function in unit.get("functions", []):
                out[(key, function.get("function"))] = function.get("status")
        return out

    fb, fa = functions(units_before), functions(units_after)
    print(f"projected functions: {len(fb)} -> {len(fa)}")
    function_moves = [
        (key, fb[key], fa[key]) for key in sorted(set(fb) & set(fa)) if fb[key] != fa[key]
    ]
    print(f"function status moves: {len(function_moves)}")
    for move in function_moves:
        print(f"  FUNCTION {move[0]}: {move[1]} -> {move[2]}")

    # Diagnostics.
    def counts(units: dict) -> collections.Counter:
        counter: collections.Counter = collections.Counter()
        for unit in units.values():
            counter.update(unit.get("diagnostic_counts", {}))
        return counter

    cb, ca = counts(units_before), counts(units_after)
    print("corpus diagnostic counts:")
    for category in sorted(set(cb) | set(ca)):
        delta = ca[category] - cb[category]
        print(f"  {category}: {cb[category]} -> {ca[category]} ({delta:+d})")

    changed = []
    for key in sorted(set(units_before) & set(units_after)):
        one = units_before[key].get("diagnostic_counts", {})
        two = units_after[key].get("diagnostic_counts", {})
        if one != two:
            changed.append((key, one, two))
    print(f"units with diagnostic-count changes: {len(changed)}")
    for key, one, two in changed:
        deltas = {
            category: two.get(category, 0) - one.get(category, 0)
            for category in sorted(set(one) | set(two))
            if two.get(category, 0) != one.get(category, 0)
        }
        print(f"  {key}: {deltas}")

    # Per-description inventory, so each movement names a rule.
    db: collections.Counter = collections.Counter()
    da: collections.Counter = collections.Counter()
    for unit in units_before.values():
        db.update(descriptions(unit))
    for unit in units_after.values():
        da.update(descriptions(unit))
    print("description inventory deltas:")
    for entry in sorted(set(db) | set(da)):
        delta = da[entry] - db[entry]
        if delta:
            print(f"  {entry[0]} / {entry[1]!r}: {db[entry]} -> {da[entry]} ({delta:+d})")

    # Leaf-level attribution: every individual diagnostic that appeared or
    # disappeared, named with its function and preprocessed line.
    def leaves(unit: dict) -> collections.Counter:
        counter: collections.Counter = collections.Counter()
        for function in unit.get("functions", []):
            for diagnostic in function.get("diagnostics", []):
                counter[(
                    function.get("function"),
                    diagnostic.get("line"),
                    diagnostic.get("category"),
                    diagnostic.get("description"),
                )] += 1
        return counter

    print("leaf-level diff, per changed unit:")
    total_added = 0
    total_removed = 0
    for key in sorted(set(units_before) & set(units_after)):
        one = leaves(units_before[key])
        two = leaves(units_after[key])
        if one == two:
            continue
        print(f"  UNIT {key}")
        for leaf in sorted(set(one) | set(two), key=lambda leaf: (leaf[0] or "", leaf[1] or 0)):
            delta = two[leaf] - one[leaf]
            if not delta:
                continue
            sign = "+" if delta > 0 else "-"
            total_added += max(delta, 0)
            total_removed += max(-delta, 0)
            print(f"    {sign}{abs(delta)} {leaf[0]} L{leaf[1]} {leaf[2]}: {leaf[3]}")
    print(f"leaves added: {total_added}; leaves removed: {total_removed}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
