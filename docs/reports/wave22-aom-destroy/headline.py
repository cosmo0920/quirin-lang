#!/usr/bin/env python3
"""Headline histogram of one produced 185-unit artifact (or of a frozen
gzipped archive), for card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED.

Read-only. Prints the unit-status histogram, the projected-function histogram,
the diagnostic-category totals and the per-library unit split, so a BEFORE and
an AFTER run can be compared without any number being retyped by hand.

    python3 -B docs/reports/wave22-aom-destroy/headline.py measure/runs/before
    python3 -B docs/reports/wave22-aom-destroy/headline.py \
        docs/reports/wave20-corpus/artifacts/diagnostics.json.gz
"""

from __future__ import annotations

import collections
import gzip
import json
import pathlib
import sys


def load(target: pathlib.Path) -> dict:
    if target.is_dir():
        return json.loads((target / "resource-diagnostics.json").read_text())
    if target.suffix == ".gz":
        return json.loads(gzip.open(target).read())
    return json.loads(target.read_text())


def main() -> int:
    for argument in sys.argv[1:]:
        document = load(pathlib.Path(argument))
        units = document["units"]
        unit_status: collections.Counter = collections.Counter()
        function_status: collections.Counter = collections.Counter()
        categories: collections.Counter = collections.Counter()
        libraries: dict = collections.defaultdict(collections.Counter)
        functions = 0
        for unit in units:
            unit_status[unit.get("status")] += 1
            libraries[unit.get("library")][unit.get("status")] += 1
            for function in unit.get("functions", []):
                functions += 1
                function_status[function.get("status")] += 1
                for diagnostic in function.get("diagnostics", []):
                    categories[diagnostic.get("category")] += 1
        print(f"== {argument}")
        print(f"   units               : {len(units)}  {dict(sorted(unit_status.items(), key=lambda item: str(item[0])))}")
        print(f"   projected functions : {functions}  {dict(sorted(function_status.items(), key=lambda item: str(item[0])))}")
        print(f"   diagnostics         : {dict(sorted(categories.items(), key=lambda item: str(item[0])))}")
        for library in sorted(libraries):
            print(f"   {library:20s}: {dict(sorted(libraries[library].items(), key=lambda item: str(item[0])))}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
