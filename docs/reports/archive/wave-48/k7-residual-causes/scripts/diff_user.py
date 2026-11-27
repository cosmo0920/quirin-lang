#!/usr/bin/env python3
"""diff_user.py RUN_DIR USER_REPORT -- K7: rows of a unit-scoped run that
differ from the user's 11-24 rows of the same units (status or leaves)."""
import collections
import json
import sys


def rows(path, files=None):
    report = json.load(open(path))
    out = {}
    for unit in report["units"]:
        if files is not None and unit["file"] not in files:
            continue
        for function in unit["functions"]:
            out[(unit["file"], function["function"])] = (
                function["status"],
                collections.Counter((d["category"], d["description"], d["line"]) for d in function["diagnostics"]))
    return out


run = rows(sys.argv[1] + "/resource-diagnostics.json")
user = rows(sys.argv[2], {f for f, _ in run})
for key in sorted(set(run) | set(user)):
    a, b = user.get(key), run.get(key)
    if a != b:
        print(key, "user", a and a[0], "run", b and b[0])
        if a and b:
            print("   removed", list((a[1] - b[1]).elements()))
            print("   added", list((b[1] - a[1]).elements()))
