#!/usr/bin/env python3
"""CG16-INDEX-AND-OFFSET-EXTENTS: summarise one run's bounds selection.

usage: bounds_summary.py BOUNDS_JSONL [CUSTODY_REPORT]

Reads the JSON lines `QCORE_BOUNDS_PROPERTY` wrote and prints, as JSON: the
bounds, custody and combined tallies over the projected functions (Unknown and
Pending in every denominator), the functions per reason code, every Unsafe
bounds row with its violating records (for classification by reading), the
obligation kinds and their outcomes, and the custody/bounds cross tabulation.
With CUSTODY_REPORT (the run's resource-diagnostics.json) it also checks that
every bounds row's custody status is the published one.
"""

import collections
import json
import sys


def main():
    rows = [json.loads(line) for line in open(sys.argv[1], encoding="utf-8") if line.strip()]
    tally = {key: collections.Counter() for key in ("bounds", "custody", "combined")}
    cross = collections.Counter()
    codes = collections.Counter()
    kinds = collections.Counter()
    unsafe = []
    for row in rows:
        bounds = (row.get("bounds") or {}).get("verdict", "absent")
        custody = row.get("custody") or "absent"
        tally["bounds"][bounds] += 1
        tally["custody"][custody] += 1
        tally["combined"][row["combined"]] += 1
        cross[f"{custody} x {bounds}"] += 1
        seen = set()
        for reason in (row.get("bounds") or {}).get("reasons", []):
            seen.add(reason.split(":")[0])
        for code in seen:
            codes[code] += 1
        for access in (row.get("bounds") or {}).get("accesses", []):
            outcome = access["actual"].split(":")[0]
            kinds[f"{access['kind']} / {outcome}"] += 1
        if bounds == "Unsafe":
            unsafe.append({
                "library": row["library"],
                "file": row["file"],
                "function": row["function"],
                "violations": [a for a in row["bounds"]["accesses"] if a["actual"] == "violation"],
            })
    summary = {
        "functions": len(rows),
        "tally": {k: dict(sorted(v.items())) for k, v in tally.items()},
        "custody_x_bounds": dict(sorted(cross.items())),
        "functions_by_reason_code": dict(codes.most_common()),
        "obligations_by_kind_and_outcome": dict(sorted(kinds.items())),
        "unsafe": unsafe,
    }
    if len(sys.argv) > 2:
        report = json.load(open(sys.argv[2], encoding="utf-8"))
        published = {}
        for unit in report["units"]:
            for function in unit.get("functions", []):
                published[(unit.get("library"), unit.get("file"), function["function"])] = function.get("status")
        mismatched = [
            r["function"] for r in rows
            if (r["library"], r["file"], r["function"]) in published
            and published[(r["library"], r["file"], r["function"])] != r["custody"]
        ]
        summary["custody_rows_matched"] = sum(
            1 for r in rows if (r["library"], r["file"], r["function"]) in published
        )
        summary["custody_mismatches"] = mismatched
    json.dump(summary, sys.stdout, indent=1, ensure_ascii=False)
    print()


if __name__ == "__main__":
    main()
