#!/usr/bin/env python3
"""Function and unit status totals, and category counts, of one or more runs.

Usage: totals.py RUN_DIR|ARTIFACT_DIR...

Reads `resource-diagnostics.json` or `diagnostics.json.gz` from each directory.
"""

import collections
import gzip
import json
import pathlib
import sys


def load(directory):
    directory = pathlib.Path(directory)
    raw = directory / "resource-diagnostics.json"
    if raw.exists():
        return json.loads(raw.read_text())
    return json.load(gzip.open(directory / "diagnostics.json.gz"))


def main() -> int:
    record = {}
    for directory in sys.argv[1:]:
        data = load(directory)
        functions = collections.Counter(f["status"] for u in data["units"] for f in u["functions"])
        units = collections.Counter(u["status"] for u in data["units"])
        record[directory] = {
            "functions": dict(functions),
            "units": dict(units),
            "diagnostic_counts": data.get("diagnostic_counts"),
        }
    print(json.dumps(record, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
