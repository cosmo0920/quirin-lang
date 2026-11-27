#!/usr/bin/env python3
"""Headline tallies of one corpus run: function and unit statuses, category counts,
and the `call to uncontracted function` leaf/function counts.

Usage: tallies.py RUN_OR_ARTIFACT_DIR [...]
"""

import collections
import gzip
import json
import pathlib
import sys


def load(path: pathlib.Path):
    if (path / "diagnostics.json.gz").exists():
        return json.load(gzip.open(path / "diagnostics.json.gz"))
    return json.loads((path / "resource-diagnostics.json").read_text())


def tally(data):
    functions = collections.Counter()
    units = collections.Counter()
    categories = collections.Counter()
    unc_leaves = 0
    unc_functions = 0
    for unit in data["units"]:
        units[unit.get("status")] += 1
        for entry in unit["functions"]:
            functions[entry.get("status")] += 1
            has_unc = False
            for diag in entry.get("diagnostics") or []:
                categories[diag["category"]] += 1
                if diag["description"].startswith("call to uncontracted function"):
                    unc_leaves += 1
                    has_unc = True
            unc_functions += has_unc
    return {
        "functions": dict(sorted(functions.items())),
        "units": dict(sorted(units.items(), key=lambda kv: str(kv[0]))),
        "categories": dict(sorted(categories.items())),
        "uncontracted_call_leaves": unc_leaves,
        "functions_with_uncontracted_call": unc_functions,
    }


def main() -> int:
    out = {arg: tally(load(pathlib.Path(arg))) for arg in sys.argv[1:]}
    print(json.dumps(out, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
