#!/usr/bin/env python3
"""Unique projected functions of a system-slice report, by (file, function).

Usage: system_unique.py REPORT.json [REPORT.json ...]

A function compiled in several configurations is joined over them
(Safe < Pending < Unknown < Unsafe), as the W46 records count them.
"""

import collections
import json
import sys

ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}

for path in sys.argv[1:]:
    report = json.load(open(path))
    joined = {}
    for unit in report["units"]:
        for row in unit.get("functions", []):
            key = (unit["file"], row["function"])
            status = row["status"]
            if key not in joined or ORDER[status] > ORDER[joined[key]]:
                joined[key] = status
    print(path, len(joined), dict(collections.Counter(joined.values())))
