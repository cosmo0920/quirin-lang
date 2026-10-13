#!/usr/bin/env python3
"""totals.py -- HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING: custody and null
totals of each BEFORE/AFTER route from the null-property summaries, with each
report's digest and the binary digest its run recorded."""
import hashlib
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
PRIVATE = REPO / "private/hur"
SLICES = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
for side in ("before", "after"):
    reports = {
        "corpus": PRIVATE / side / "resource-diagnostics.json",
        "system": PRIVATE / f"{side}-system/resource-diagnostics.json",
        "slice": SLICES / f"hur-{side}/resource-diagnostics.json",
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
