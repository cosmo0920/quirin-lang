#!/usr/bin/env python3
"""Summarise the generic-selection census per function, joined with BEFORE.

Usage: census_table.py CENSUS_JSON BEFORE_DIAGNOSTICS OUT_JSON
"""
import collections
import json
import pathlib
import sys

census = json.loads(pathlib.Path(sys.argv[1]).read_text())
before = json.loads(pathlib.Path(sys.argv[2]).read_text())
status = {}
generic_leaves = collections.Counter()
leaf_count = {}
for unit in before["units"]:
    for function in unit["functions"]:
        key = (unit["file"], function["function"])
        status[key] = function["status"]
        leaf_count[key] = len(function["diagnostics"])
        for leaf in function["diagnostics"]:
            if "_Generic" in leaf["description"]:
                generic_leaves[key] += 1
per = collections.defaultdict(lambda: {"selections": 0, "positions": collections.Counter(),
                                       "callees": collections.Counter(), "refusals": collections.Counter()})
for row in census["rows"]:
    unit = row["units"][0] if row["units"] else None
    key = (unit, row["function"])
    entry = per[key]
    entry["selections"] += 1
    entry["positions"][row["position"]] += 1
    entry["callees"][row["shape"].split(":")[-1]] += 1
    for refusals in row["refusals_at_line"].values():
        for refusal in set(refusals):
            entry["refusals"][refusal] += 1
table = []
for (unit, function), entry in sorted(per.items(), key=lambda kv: (str(kv[0][0]), str(kv[0][1]))):
    key = (unit, function)
    table.append({
        "unit": unit, "function": function,
        "projected": key in status,
        "status_before": status.get(key),
        "leaves_before": leaf_count.get(key),
        "generic_leaves_before": generic_leaves.get(key, 0),
        "selections": entry["selections"],
        "positions": dict(entry["positions"]),
        "callees": dict(entry["callees"]),
        "refusals_at_selection_statements": dict(entry["refusals"]),
    })
missing = [f"{u}:{f}" for (u, f), n in generic_leaves.items() if (u, f) not in per]
summary = {
    "functions_with_selections": len(table),
    "projected_functions_with_selections": sum(1 for t in table if t["projected"]),
    "units_with_selections": len({t["unit"] for t in table}),
    "status_before": dict(collections.Counter(t["status_before"] for t in table)),
    "generic_leaves_before": sum(generic_leaves.values()),
    "generic_leaf_functions_before": len(generic_leaves),
    "generic_leaf_functions_missing_from_census": missing,
    "refusal_kinds_at_selection_statements": dict(sum((collections.Counter(t["refusals_at_selection_statements"]) for t in table), collections.Counter())),
    "selections_with_no_leaf_at_statement": sum(1 for r in census["rows"] if not any(r["refusals_at_line"].values())),
}
pathlib.Path(sys.argv[3]).write_text(json.dumps({"summary": summary, "functions": table}, indent=1) + "\n")
print(json.dumps(summary, indent=1))
for t in table:
    print(t["unit"], t["function"], t["status_before"], t["leaves_before"], t["generic_leaves_before"], t["selections"], t["positions"])
