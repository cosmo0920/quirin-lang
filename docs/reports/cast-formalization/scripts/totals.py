#!/usr/bin/env python3
"""CAST-FORMALIZATION: each report's sha256, status totals and leaf counts
(usage: totals.py LABEL=REPORT.json ...)."""

import collections
import hashlib
import json
import sys

for argument in sys.argv[1:]:
    label, path = argument.split("=", 1)
    raw = open(path, "rb").read()
    data = json.loads(raw)
    statuses = collections.Counter(
        entry["status"] for unit in data["units"] for entry in unit["functions"])
    print("%s %s Safe %d / Unsafe %d / Unknown %d / Pending %d; %s" % (
        label, hashlib.sha256(raw).hexdigest(), statuses["Safe"], statuses["Unsafe"],
        statuses["Unknown"], statuses["Pending"], data.get("diagnostic_counts")))
