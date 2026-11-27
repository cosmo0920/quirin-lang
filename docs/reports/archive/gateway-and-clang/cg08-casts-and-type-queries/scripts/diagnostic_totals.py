#!/usr/bin/env python3
"""CG08-CASTS-AND-TYPE-QUERIES: corpus diagnostic totals by category.

usage: diagnostic_totals.py RUN...   (run directories)
"""
import collections
import json
import pathlib
import sys

for run in sys.argv[1:]:
    data = json.loads((pathlib.Path(run) / "resource-diagnostics.json").read_text())
    totals = collections.Counter()
    for unit in data["units"]:
        for function in unit["functions"]:
            for diagnostic in function.get("diagnostics", []):
                totals[diagnostic.get("category")] += 1
    print(run, dict(sorted(totals.items())))
