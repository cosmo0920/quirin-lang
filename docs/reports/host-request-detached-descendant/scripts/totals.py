#!/usr/bin/env python3
"""totals.py -- HOST-REQUEST-READ-BY-DETACHED-DESCENDANT (from HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING): custody and null
totals of each BEFORE/AFTER route from the null-property summaries, with each
report's digest and the binary digest its run recorded."""
import hashlib
import json
import sys
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
PRIVATE = REPO / "private/hdd"
SLICES = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
for side in ["before", *(sys.argv[1:] or ["after"])]:
    reports = {
        "corpus": PRIVATE / side / "resource-diagnostics.json",
        "system": PRIVATE / f"{side}-system/resource-diagnostics.json",
        "slice": SLICES / f"hdd-{side}/resource-diagnostics.json",
    }
    summaries = {
        "corpus": PRIVATE / f"{side}-null.summary.json",
        "system": PRIVATE / f"{side}-system-null.summary.json",
        "slice": PRIVATE / f"{side}-slice-null.summary.json",
    }
    run = json.loads((PRIVATE / side / "run.json").read_text())
    print(f"{side}: corpus run binary {run['binary_sha256'][:16]}")
    for route, report in reports.items():
        summary = json.loads(summaries[route].read_text())
        digest = hashlib.sha256(report.read_bytes()).hexdigest()[:8]
        print(f"  {route}: custody {summary['custody']} report {digest}; null {summary['null']}")
