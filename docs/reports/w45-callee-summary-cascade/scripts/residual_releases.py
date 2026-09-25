#!/usr/bin/env python3
"""Which recognised pointer actuals at a `Released` position still refuse AFTER.

Usage: residual_releases.py CALL_ARGUMENT_CENSUS.jsonl CENSUS_DIAGNOSTICS.json AFTER_DIAGNOSTICS.json OUT.json

The census run analyses one unit at a time, so its `unit_seq` N is the N-th unit
of its own diagnostics. Each distinct (unit, function, line) whose statement has
a recognised pointer actual at a `released` position is looked up in AFTER: if
the line still carries `indirect memory access`, rule 1 did not admit it (a
non-null release, another refused actual in the statement, an uncontracted or
non-release callee, or a failed obligation). The callee's contract nullability
is not in the census, so the residual is reported per callee for reading.
"""
import collections
import json
import sys


def main():
    census, census_diag, after_diag, out = sys.argv[1:5]
    units = [u["library"] + "/" + u["file"] for u in json.load(open(census_diag))["units"]]
    after = json.load(open(after_diag))
    refused = set()
    for unit in after["units"]:
        key = unit["library"] + "/" + unit["file"]
        for entry in unit["functions"]:
            for d in entry["diagnostics"]:
                if d["description"].startswith("indirect memory access"):
                    refused.add((key, entry["function"], d["line"]))
    sites = {}
    for line in open(census):
        record = json.loads(line)
        if not any(a["access"] and a["position"] == "released" and a["recognised"] == "pointer"
                   for a in record["arguments"]):
            continue
        unit = units[record["unit_seq"] - 1]
        sites[(unit, record["function"], record["line"])] = record["callee"]
    still = {k: v for k, v in sites.items() if k in refused}
    result = {
        "statements_with_a_recognised_released_pointer": len(sites),
        "still_refused_after": len(still),
        "admitted_after": len(sites) - len(still),
        "still_refused_by_callee": collections.Counter(still.values()).most_common(),
    }
    json.dump(result, open(out, "w"), indent=1)
    print(json.dumps({k: v for k, v in result.items() if k != "still_refused_by_callee"}, indent=1))
    print(result["still_refused_by_callee"][:25])


if __name__ == "__main__":
    main()
