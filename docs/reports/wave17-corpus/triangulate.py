#!/usr/bin/env python3
"""Wave-17 consolidation: attribute the composed diagnostics leaf set against
the one card that moved diagnostics, and check the two byte-identity claims.

Read-only, and reproducible from committed bytes: every input is an archived
artifact set. Three of wave 17's eight cards touched the gateway's decision
surface; only CONSTRUCTOR-RESIDUALS measured a non-empty corpus diff. Cards
CTWIN-RESIDUALS and LOADER-ABI-MODES each measured all five artifacts
byte-identical between their own BEFORE and AFTER runs, so the composed tree
must differ from the wave-16 baseline by exactly CONSTRUCTOR-RESIDUALS' leaves
and by nothing else. This asserts that rather than arguing it.

    python3 -B docs/reports/wave17-corpus/triangulate.py
"""
from __future__ import annotations

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]

BASELINE = ROOT / "docs/reports/wave16-corpus/artifacts"
CONSTRUCTOR = ROOT / "docs/reports/remaining-constructor-residuals/artifacts"
COMPOSED = ROOT / "docs/reports/wave17-corpus/artifacts"


def load_gz(path: pathlib.Path) -> dict:
    with gzip.open(path, "rt") as handle:
        return json.load(handle)


def leaves(report: dict) -> collections.Counter:
    counter: collections.Counter = collections.Counter()
    for unit in report["units"]:
        key = f"{unit.get('library')}/{unit.get('file')}"
        for function in unit.get("functions", []):
            name = function.get("function") or function.get("name")
            for diagnostic in function.get("diagnostics", []):
                counter[(key, name, diagnostic.get("line"),
                         diagnostic.get("category"), diagnostic.get("description"))] += 1
    return counter


def statuses(report: dict) -> dict:
    return {f"{unit.get('library')}/{unit.get('file')}": unit.get("status")
            for unit in report["units"]}


def function_statuses(report: dict) -> dict:
    out = {}
    for unit in report["units"]:
        key = f"{unit.get('library')}/{unit.get('file')}"
        for function in unit.get("functions", []):
            out[(key, function.get("function") or function.get("name"))] = function.get("status")
    return out


def diff(before: collections.Counter, after: collections.Counter) -> tuple[collections.Counter, collections.Counter]:
    added = collections.Counter()
    removed = collections.Counter()
    for leaf in set(before) | set(after):
        delta = after[leaf] - before[leaf]
        if delta > 0:
            added[leaf] = delta
        elif delta < 0:
            removed[leaf] = -delta
    return added, removed


def main() -> int:
    baseline = load_gz(BASELINE / "diagnostics.json.gz")
    constructor = load_gz(CONSTRUCTOR / "diagnostics.json.gz")
    composed = load_gz(COMPOSED / "diagnostics.json.gz")

    base_leaves = leaves(baseline)
    card_leaves = leaves(constructor)
    composed_leaves = leaves(composed)

    card_added, card_removed = diff(base_leaves, card_leaves)
    composed_added, composed_removed = diff(base_leaves, composed_leaves)

    print("wave-16 baseline -> CONSTRUCTOR-RESIDUALS archived AFTER: "
          f"added {sum(card_added.values())}, removed {sum(card_removed.values())}")
    print("wave-16 baseline -> composed wave-17 measurement:          "
          f"added {sum(composed_added.values())}, removed {sum(composed_removed.values())}")

    same = (card_added == composed_added) and (card_removed == composed_removed)
    print(f"composed diff == CONSTRUCTOR-RESIDUALS' diff: {same}")

    unattributed_added = composed_added - card_added
    unattributed_removed = composed_removed - card_removed
    print(f"unattributed added leaves: {sum(unattributed_added.values())}")
    print(f"unattributed removed leaves: {sum(unattributed_removed.values())}")
    for leaf, count in sorted(unattributed_added.items(), key=lambda item: str(item[0])):
        print(f"  UNATTRIBUTED +{count} {leaf}")
    for leaf, count in sorted(unattributed_removed.items(), key=lambda item: str(item[0])):
        print(f"  UNATTRIBUTED -{count} {leaf}")

    # CTWIN-RESIDUALS and LOADER-ABI-MODES each claimed zero corpus reach.
    print(f"zero leaves attributable to CTWIN-RESIDUALS or LOADER-ABI-MODES: "
          f"{sum(unattributed_added.values()) == 0 and sum(unattributed_removed.values()) == 0}")

    before_status = statuses(baseline)
    after_status = statuses(composed)
    moves = {key: (before_status[key], after_status[key])
             for key in before_status if before_status.get(key) != after_status.get(key)}
    print(f"unit status moves: {len(moves)}")
    for key, pair in sorted(moves.items()):
        print(f"  MOVED {key}: {pair[0]} -> {pair[1]}")

    before_functions = function_statuses(baseline)
    after_functions = function_statuses(composed)
    function_moves = {key: (before_functions[key], after_functions.get(key))
                      for key in before_functions
                      if before_functions[key] != after_functions.get(key)}
    print(f"projected functions: {len(before_functions)} -> {len(after_functions)}")
    print(f"function status moves: {len(function_moves)}")
    for key, pair in sorted(function_moves.items(), key=lambda item: str(item[0])):
        print(f"  MOVED {key}: {pair[0]} -> {pair[1]}")

    histogram_before = collections.Counter(before_status.values())
    histogram_after = collections.Counter(after_status.values())
    print(f"unit status histogram before: {dict(sorted(histogram_before.items()))}")
    print(f"unit status histogram after:  {dict(sorted(histogram_after.items()))}")

    categories_before: collections.Counter = collections.Counter()
    categories_after: collections.Counter = collections.Counter()
    for leaf, count in base_leaves.items():
        categories_before[leaf[3]] += count
    for leaf, count in composed_leaves.items():
        categories_after[leaf[3]] += count
    for category in sorted(set(categories_before) | set(categories_after)):
        before_count = categories_before[category]
        after_count = categories_after[category]
        if before_count != after_count:
            print(f"category {category}: {before_count} -> {after_count} "
                  f"({after_count - before_count:+d})")

    ok = same and not moves and not function_moves
    print(f"RESULT: {'attributed' if ok else 'NOT ATTRIBUTED'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
