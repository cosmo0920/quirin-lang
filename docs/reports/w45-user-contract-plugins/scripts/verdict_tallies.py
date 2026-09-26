#!/usr/bin/env python3
"""Tally an arbiter run's unit and function verdicts over the four values.

usage: verdict_tallies.py RUN_DIR COMMIT

Reads `RUN_DIR/resource-diagnostics.json` and prints the unit and function
status counts, `Pending` included (card W45-USER-CONTRACT-PLUGINS).
"""

import collections
import json
import pathlib
import sys


def main() -> int:
    run = pathlib.Path(sys.argv[1])
    report = json.loads((run / "resource-diagnostics.json").read_text())
    units = collections.Counter(unit["status"] for unit in report["units"])
    functions = collections.Counter(
        function["status"] for unit in report["units"] for function in unit["functions"])
    print(json.dumps({
        "run": str(run),
        "commit": sys.argv[2],
        "units": dict(sorted(units.items())),
        "functions": dict(sorted(functions.items())),
        "pending_units": units.get("Pending", 0),
        "pending_functions": functions.get("Pending", 0),
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
