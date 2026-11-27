#!/usr/bin/env python3
"""lookup_current.py REPORT FUNCTION...: print the rows and leaves of the named
functions from a resource-diagnostics report (read-only)."""
import json
import sys

report = sys.argv[1]
want = set(sys.argv[2:])
data = json.load(open(report))
for unit in data["units"]:
    for function in unit.get("functions", []):
        if function["function"] in want:
            print(unit.get("library"), unit.get("file"), function["function"], function["status"])
            for leaf in function.get("diagnostics", []):
                print("   ", leaf.get("category"), leaf.get("description"), leaf.get("origin"))
