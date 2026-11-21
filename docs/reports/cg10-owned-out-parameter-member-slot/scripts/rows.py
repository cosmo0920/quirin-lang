#!/usr/bin/env python3
"""rows.py REPORT [NAME...] -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4: the status and leaves of
the named function rows of one `current` report (every row without names)."""
import json, sys
data = json.load(open(sys.argv[1]))
names = set(sys.argv[2:])
for unit in data["units"]:
    for f in unit["functions"]:
        if f["function"] in names or (not names):
            print(unit.get("file", unit.get("path")), f["function"], f["status"])
            for d in f["diagnostics"]:
                print("   ", d.get("line"), d["description"][:160])
