#!/usr/bin/env python3
"""Print the full report row (every field but the diagnostics, then the
diagnostics) of the named functions.

Usage: rows.py RESOURCE_DIAGNOSTICS_JSON NAME [NAME ...]
"""
import json
import sys

data = json.load(open(sys.argv[1]))
wanted = set(sys.argv[2:])
for unit in data["units"]:
    for row in unit.get("functions", []):
        if row["function"] in wanted:
            print(unit["file"].split("/")[-1], json.dumps(row, indent=1)[:4000])
