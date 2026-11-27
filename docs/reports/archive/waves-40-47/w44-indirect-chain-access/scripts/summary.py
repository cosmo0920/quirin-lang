#!/usr/bin/env python3
"""Card W44-INDIRECT-CHAIN-ACCESS: headline figures of one audit run.

Usage: summary.py RUN_DIR [IDENTITY_JSON]

Prints function and unit verdict counts, the diagnostic-kind counts (leak,
unknown_alias, unsupported, ...) over every function, and, when an identity
record from `baseline_identity.py` is given, its per-artifact verdicts.
"""

import collections
import json
import pathlib
import sys


def figures(run):
    doc = json.loads((pathlib.Path(run) / "resource-diagnostics.json").read_text())
    functions = collections.Counter()
    units = collections.Counter()
    kinds = collections.Counter()
    for unit in doc["units"]:
        units[unit.get("status")] += 1
        for function in unit["functions"]:
            functions[function["status"]] += 1
            for diagnostic in function["diagnostics"]:
                kinds[diagnostic.get("category")] += 1
    return {"functions": dict(functions), "units": dict(units), "kinds": dict(kinds)}


def main():
    result = figures(sys.argv[1])
    if len(sys.argv) > 2:
        identity = json.loads(pathlib.Path(sys.argv[2]).read_text())
        result["identity"] = {
            label: {key: value for key, value in entry.items() if "sha" not in key}
            for label, entry in identity["artifacts"].items()
        }
    print(json.dumps(result, indent=1, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
