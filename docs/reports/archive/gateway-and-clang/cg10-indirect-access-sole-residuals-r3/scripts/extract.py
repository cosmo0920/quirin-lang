#!/usr/bin/env python3
"""extract.py REPORT OUT.json -- AG1-INDIRECT-MEMBER-ACCESS census, step 1.

Reads a `quirin-audit current` resource-diagnostics report (read-only) and
writes, for every Unknown function that carries at least one
`indirect memory access is outside the resource projection` leaf, its unit,
library, function, every leaf (category, description, line, origin) and
whether the access refusal is its only blocker. Nothing is written next to
the report."""
import json
import sys

GATE = "indirect memory access is outside the resource projection"


def main() -> int:
    report, out = sys.argv[1], sys.argv[2]
    data = json.load(open(report))
    rows = []
    totals = {}
    for unit in data["units"]:
        for function in unit.get("functions", []):
            totals[function["status"]] = totals.get(function["status"], 0) + 1
            if function["status"] != "Unknown":
                continue
            leaves = function.get("diagnostics", [])
            access = [leaf for leaf in leaves if leaf["description"] == GATE]
            if not access:
                continue
            others = [leaf for leaf in leaves if leaf["description"] != GATE]
            rows.append(
                {
                    "unit": unit["file"],
                    "library": unit.get("library"),
                    "function": function["function"],
                    "access": access,
                    "others": others,
                    "sole": not others,
                }
            )
    with open(out, "w") as handle:
        json.dump({"totals": totals, "rows": rows}, handle, indent=0)
    print(totals, len(rows), sum(row["sole"] for row in rows))
    return 0


if __name__ == "__main__":
    sys.exit(main())
