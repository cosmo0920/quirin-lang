#!/usr/bin/env python3
"""Status of named functions in the report (reading aid).
usage: callee_status.py REPORT.json NAME..."""
import collections
import json
import sys

report = json.load(open(sys.argv[1]))
names = set(sys.argv[2:])
for unit in report["units"]:
    for function in unit["functions"]:
        if function["function"] in names:
            cats = collections.Counter(d["category"] for d in function["diagnostics"])
            print(unit["file"], function["function"], function["status"], dict(cats))
            if function["status"] != "Safe":
                for d in function["diagnostics"][:6]:
                    print("    ", d["category"], d["description"][:150], d.get("origin"))
