#!/usr/bin/env python3
"""CG13: rule 10w's decisions in a parity stream (`position: union-bitfield`).

usage: punned.py PARITY_JSONL OUT_JSON

Counts consultations deduplicated by (function, node offset, admitted,
outcomes): a statement is consulted by several positions and rebuilt by the
W4 re-check, and each consultation writes a row. Reports the admitted and
refused consultations by the outcome of each step, and the punned refusals'
functions.
"""

import collections
import json
import sys


def main():
    seen = set()
    outcomes = collections.Counter()
    admitted = collections.Counter()
    punned_functions = set()
    punned = 0
    with open(sys.argv[1]) as handle:
        for line in handle:
            if '"union-bitfield"' not in line:
                continue
            row = json.loads(line)
            if row.get("position") != "union-bitfield":
                continue
            steps = tuple((s["storage"], s["access"], s["outcome"]) for s in row["steps"])
            key = (row.get("function"), row.get("offset"), row["admitted"], steps)
            if key in seen:
                continue
            seen.add(key)
            admitted[row["admitted"]] += 1
            for storage, access, outcome in steps:
                outcomes[f"{storage} {access} {outcome}"] += 1
            if any(outcome == "refused-punned" for _, _, outcome in steps):
                punned += 1
                punned_functions.add(row.get("function"))
    result = {
        "schema": "qcore.cg13-union-bitfield-decisions.v1",
        "consultations": punned,
        "functions": len(punned_functions),
        "all_consultations": {"admitted": admitted[True], "refused": admitted[False]},
        "step_outcomes": dict(outcomes.most_common()),
        "punned_functions": sorted(f for f in punned_functions if f),
    }
    with open(sys.argv[2], "w") as handle:
        json.dump(result, handle, indent=1, sort_keys=True)
        handle.write("\n")
    print(json.dumps({k: v for k, v in result.items() if k != "punned_functions"}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
