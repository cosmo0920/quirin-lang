#!/usr/bin/env python3
"""totals.py REPORT... -- GAPO-RETURN-CORRECTNESS (from QR-LEXICAL-SELECTORS-DECLARATION-NAMES and its lineage): per route report (a
run directory or its resource-diagnostics.json): Safe / Unsafe / Unknown
(and Pending) row counts and the report's sha256."""
import collections
import hashlib
import json
import pathlib
import sys

for argument in sys.argv[1:]:
    path = pathlib.Path(argument)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    data = path.read_bytes()
    report = json.loads(data)
    counts = collections.Counter(f["status"] for u in report["units"] for f in u["functions"])
    print("%s Safe %d Unsafe %d Unknown %d Pending %d sha256 %s" % (
        argument, counts["Safe"], counts["Unsafe"], counts["Unknown"], counts["Pending"],
        hashlib.sha256(data).hexdigest()))
