#!/usr/bin/env python3
"""Blocker figures of one corpus report, in the STATUS rows' vocabulary.

Usage: blockers.py REPORT.json

For each family: leaves on Unknown functions, those functions, and the
functions it is the sole blocker of (every other leaf of the function is in
the same family). Families: uncontracted calls, branch-condition refusals
(`in branch condition` exactly), every controlling-expression refusal,
indirect access, return expression; plus the unit reason codes.
"""

import collections
import json
import sys

FAMILIES = {
    "uncontracted": lambda d: d["description"].startswith("call to uncontracted function"),
    "branch-condition": lambda d: d["description"] == "unmodeled call, aggregate or indirect access in branch condition",
    "any-condition": lambda d: d["description"].startswith("unmodeled call, aggregate or indirect access in")
    and "return expression" not in d["description"],
    "indirect-access": lambda d: d["description"] == "indirect memory access is outside the resource projection",
    "return-expression": lambda d: d["description"] == "unmodeled call, aggregate or indirect access in return expression",
}

report = json.load(open(sys.argv[1]))
result = {}
for name, test in FAMILIES.items():
    leaves = functions = sole = 0
    for unit in report["units"]:
        for row in unit.get("functions", []):
            if row["status"] != "Unknown":
                continue
            diagnostics = row.get("diagnostics", [])
            hits = [d for d in diagnostics if d["category"] == "unsupported" and test(d)]
            if hits:
                leaves += len(hits)
                functions += 1
                if len(hits) == len(diagnostics):
                    sole += 1
    result[name] = {"leaves": leaves, "unknown_functions": functions, "sole": sole}
result["unit_reason_codes"] = dict(collections.Counter(u.get("reason_code") or "none" for u in report["units"]))
print(json.dumps(result, indent=1, sort_keys=True))
