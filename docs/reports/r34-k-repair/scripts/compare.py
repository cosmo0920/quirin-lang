#!/usr/bin/env python3
"""compare.py BASE_REPORT TIP_REPORT: status moves between two reports, by
(file, function), and the totals of each."""
import collections
import json
import sys


def rows(path):
    out = {}
    for unit in json.load(open(path))["units"]:
        for function in unit.get("functions", []):
            out[(unit.get("file"), function["function"])] = function["status"]
    return out


base, tip = rows(sys.argv[1]), rows(sys.argv[2])
print("base", dict(collections.Counter(base.values())), "tip", dict(collections.Counter(tip.values())))
for key in sorted(set(base) | set(tip)):
    if base.get(key) != tip.get(key):
        print(f"{key[0]}::{key[1]} {base.get(key)} -> {tip.get(key)}")
