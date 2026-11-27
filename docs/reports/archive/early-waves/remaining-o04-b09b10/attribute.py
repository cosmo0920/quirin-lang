#!/usr/bin/env python3
"""Per-unit, per-diagnostic attribution of the corpus movement for card
O04-B09B10.

Read-only. Prints, for every unit whose diagnostic inventory moved, the exact
set of diagnostics that appeared and disappeared, keyed by (function, line,
kind, description), so every movement is attributed to a statement rather than
argued.

    python3 -B docs/reports/remaining-o04-b09b10/attribute.py \\
        --before build/cycle19/reports-before \\
        --after  build/cycle19/reports-after
"""

from __future__ import annotations

import argparse
import collections
import json
import pathlib

DIAGNOSTICS = "resource-diagnostics.json"


def load(directory: pathlib.Path) -> dict:
    return json.loads((directory / DIAGNOSTICS).read_text())


def unit_key(unit: dict) -> str:
    return f"{unit.get('library')}/{unit.get('file')}"


def rows(unit: dict) -> collections.Counter:
    counter: collections.Counter = collections.Counter()
    for function in unit.get("functions", []):
        for diagnostic in function.get("diagnostics", []):
            counter[
                (
                    function.get("function"),
                    diagnostic.get("line"),
                    diagnostic.get("kind"),
                    diagnostic.get("description") or diagnostic.get("variable") or "",
                )
            ] += 1
    return counter


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--before", type=pathlib.Path, required=True)
    parser.add_argument("--after", type=pathlib.Path, required=True)
    arguments = parser.parse_args()
    before = {unit_key(u): u for u in load(arguments.before)["units"]}
    after = {unit_key(u): u for u in load(arguments.after)["units"]}
    moved = 0
    for key in sorted(set(before) | set(after)):
        b = rows(before.get(key, {}))
        a = rows(after.get(key, {}))
        if b == a:
            continue
        moved += 1
        print(f"== {key}")
        for row, count in sorted((a - b).items()):
            print(f"  + x{count} {row}")
        for row, count in sorted((b - a).items()):
            print(f"  - x{count} {row}")
    print(f"units with diagnostic movement: {moved}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
