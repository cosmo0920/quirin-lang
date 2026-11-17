#!/usr/bin/env python3
"""Compare two `current` reports row by row and attribute every move.

    python3 compare.py BEFORE.json AFTER.json > comparison.json

A row is one function of one unit (`library`, `file`, the unit's occurrence
index among units with that library and file, `function`). For every row
whose status differs, the record names both statuses, the bound callees whose
links the after row gained or lost, the uncontracted-callee diagnostics it
lost or gained and every other diagnostic change: what attributes the move to
a declaration. Totals per status and the unit statuses are counted from each
report as written; an unmatched row is listed and fails nothing silently.
"""
import collections
import json
import re
import sys


def rows(path):
    report = json.load(open(path))
    out, units = {}, {}
    seen = collections.Counter()
    for unit in report["units"]:
        key = (unit.get("library"), unit["file"])
        index = seen[key]
        seen[key] += 1
        units[(*key, index)] = unit["status"]
        for function in unit["functions"]:
            out[(*key, index, function["function"])] = function
    return out, units


UNCONTRACTED = re.compile(r"call to uncontracted function `([^`]+)`")


def describe(function):
    diags = [d["description"] for d in function["diagnostics"]]
    return {
        "status": function["status"],
        "links": sorted({l["callee"] for l in function.get("plugin_links", [])}),
        "uncontracted": sorted({m.group(1) for d in diags for m in [UNCONTRACTED.search(d)] if m}),
        "other": sorted(d for d in diags if not UNCONTRACTED.search(d)),
        "rests_on": function.get("contract_dependencies", {}).get("declarations", []),
    }


before, before_units = rows(sys.argv[1])
after, after_units = rows(sys.argv[2])
moves, content, changes = [], 0, []
for key in sorted(set(before) & set(after), key=str):
    b, a = describe(before[key]), describe(after[key])
    if b != a:
        content += 1
    if b != a:
        (moves if b["status"] != a["status"] else changes).append({
            "row": "::".join(str(part) for part in key),
            "before": b["status"], "after": a["status"],
            "links_gained": sorted(set(a["links"]) - set(b["links"])),
            "links_lost": sorted(set(b["links"]) - set(a["links"])),
            "uncontracted_left": sorted(set(b["uncontracted"]) - set(a["uncontracted"])),
            "uncontracted_new": sorted(set(a["uncontracted"]) - set(b["uncontracted"])),
            "other_left": sorted(set(b["other"]) - set(a["other"])),
            "other_new": sorted(set(a["other"]) - set(b["other"])),
        })


def totals(table):
    return dict(sorted(collections.Counter(f["status"] for f in table.values()).items()))


json.dump({
    "before": sys.argv[1], "after": sys.argv[2],
    "function_totals": {"before": totals(before), "after": totals(after)},
    "unit_totals": {"before": dict(collections.Counter(before_units.values())),
                    "after": dict(collections.Counter(after_units.values()))},
    "unmatched": {"before_only": ["::".join(map(str, k)) for k in sorted(set(before) - set(after), key=str)],
                  "after_only": ["::".join(map(str, k)) for k in sorted(set(after) - set(before), key=str)]},
    "rows_changed": content,
    "content_changes": changes,
    "moves": moves,
    "unit_moves": [{"unit": "::".join(map(str, k)), "before": before_units[k], "after": after_units[k]}
                   for k in sorted(set(before_units) & set(after_units), key=str)
                   if before_units[k] != after_units[k]],
}, sys.stdout, indent=1)
print()
