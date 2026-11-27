#!/usr/bin/env python3
"""Print the status and every leaf of the named functions in a diagnostics file.

Usage: show.py RESOURCE_DIAGNOSTICS_JSON NAME [NAME ...]
"""
import json
import sys

diagnostics = json.load(open(sys.argv[1]))
wanted = set(sys.argv[2:])
for unit in diagnostics["units"]:
    for row in unit.get("functions", []):
        if row["function"] not in wanted:
            continue
        print(f"{row['function']} [{unit['file'].split('/')[-1]}] {row['status']} (unit {unit.get('status')})")
        for d in row.get("diagnostics", []):
            print(f"    {d.get('line')}: {d.get('category')}: {d.get('description')}")
