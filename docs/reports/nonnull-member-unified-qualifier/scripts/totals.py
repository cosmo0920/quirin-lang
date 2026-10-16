#!/usr/bin/env python3
"""totals.py REPORT.json NULL.summary.json [OUT.json] -- NONNULL-MEMBER-UNIFIED-QUALIFIER (from FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE):
one route's custody report sha256, the custody and null verdict counts and
the check outcomes from the null property's summary, and the fold's stable
and scope declarations (rule 14x) when present."""
import hashlib
import json
import sys

report_path, summary_path = sys.argv[1], sys.argv[2]
summary = json.load(open(summary_path))
fold = summary.get("fold") or {}
out = {
    "report_sha256": hashlib.sha256(open(report_path, "rb").read()).hexdigest(),
    "custody": summary["custody"],
    "null": summary["null"],
    "checks_by_outcome": summary["checks"]["by_outcome"],
    "candidates_by_source_kind": summary["checks"]["candidates_by_source_kind"],
    "stable_declarations": fold.get("stable_declarations"),
    "scope_declarations": fold.get("scope_declarations"),
    "member_declarations": fold.get("member_declarations"),
    "withdrawn": fold.get("withdrawn"),
}
text = json.dumps(out, indent=1, sort_keys=True)
if len(sys.argv) > 3:
    open(sys.argv[3], "w").write(text + "\n")
print(text)
