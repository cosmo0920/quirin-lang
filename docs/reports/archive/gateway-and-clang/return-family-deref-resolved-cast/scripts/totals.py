#!/usr/bin/env python3
"""totals.py REPORT -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4: one
opt-in `current` report's function rows by status, the Unknown rows carrying
the indirect-access refusal, those carrying nothing else, and its leaves."""
import collections
import json
import sys

GATE = "indirect memory access is outside the resource projection"


def main() -> int:
    data = json.load(open(sys.argv[1]))
    status = collections.Counter()
    rows = alone = leaves = 0
    for unit in data["units"]:
        for function in unit["functions"]:
            status[function["status"]] += 1
            if function["status"] != "Unknown":
                continue
            descriptions = [d["description"] for d in function["diagnostics"]]
            hits = sum(1 for d in descriptions if d.startswith(GATE))
            if hits:
                rows += 1
                leaves += hits
                if hits == len(descriptions):
                    alone += 1
    print(json.dumps({
        "Safe": status["Safe"], "Pending": status["Pending"], "Unknown": status["Unknown"],
        "Unsafe": status["Unsafe"], "other": {k: v for k, v in status.items()
                                               if k not in ("Safe", "Pending", "Unknown", "Unsafe")},
        "access_rows": rows, "access_alone": alone, "access_leaves": leaves,
    }))
    return 0


if __name__ == "__main__":
    sys.exit(main())
