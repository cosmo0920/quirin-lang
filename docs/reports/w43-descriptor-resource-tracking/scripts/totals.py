#!/usr/bin/env python3
"""Headline totals of one or more audit runs (raw `resource-diagnostics.json`).

Usage: totals.py RUN_DIR [RUN_DIR ...]

Prints, per run: unit statuses, function statuses, the top-level
`diagnostic_counts`, and the uncontracted-call census (leaves, Unknown
functions carrying one, Unknown functions blocked only by one callee).
"""

import collections
import json
import pathlib
import re
import sys

UNCONTRACTED = re.compile(r"call to uncontracted function `([^`]+)`")


def totals(run: pathlib.Path) -> dict:
    data = json.loads((run / "resource-diagnostics.json").read_text())
    units = collections.Counter(unit["status"] for unit in data["units"])
    functions = collections.Counter()
    leaves = functions_with = sole = 0
    for unit in data["units"]:
        for entry in unit["functions"]:
            functions[entry.get("status")] += 1
            diagnostics = entry.get("diagnostics") or []
            hits = [d for d in diagnostics if UNCONTRACTED.fullmatch(d["description"])]
            if entry.get("status") == "Unknown":
                leaves += len(hits)
                functions_with += bool(hits)
                reasons = {d["description"] for d in diagnostics}
                sole += len(reasons) == 1 and bool(hits)
    return {
        "units": dict(units),
        "functions": dict(functions),
        "diagnostic_counts": data["diagnostic_counts"],
        "uncontracted_on_unknown": {"leaves": leaves, "functions": functions_with, "sole": sole},
    }


def main() -> int:
    for argument in sys.argv[1:]:
        print(argument, json.dumps(totals(pathlib.Path(argument)), sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
