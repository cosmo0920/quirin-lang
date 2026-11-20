#!/usr/bin/env python3
"""stream_shapes.py REPORT STREAMS_DIR... -- CG10 r3: the analyzer's own
reading of every access refusal in the census-feature selection runs.

Joins each Unknown row's `indirect memory access` leaf (function, line) of
REPORT (the full opt-in run) with the `indirect.jsonl` gate sites the
selection runs wrote at that function and line, and counts leaves and rows by
the gate site's position, whether its statement holds a call and whether the
call is contracted, and the first chain's member class and base cause. A leaf
no site matches (Monkey, which the selections leave out, or a site the
census does not record) is counted apart."""
import collections
import json
import sys

GATE = "indirect memory access is outside the resource projection"


def main() -> int:
    report = json.load(open(sys.argv[1]))
    sites = collections.defaultdict(list)
    for directory in sys.argv[2:]:
        for line in open(f"{directory}/indirect.jsonl"):
            row = json.loads(line)
            sites[(row["function"], row["line"])].append(row)
    leaves = collections.Counter()
    rows = collections.defaultdict(set)
    unmatched = 0
    for unit in report["units"]:
        for function in unit.get("functions", []):
            if function["status"] != "Unknown":
                continue
            for leaf in function.get("diagnostics", []):
                if leaf["description"] != GATE:
                    continue
                found = sites.get((function["function"], leaf["line"]))
                if not found:
                    unmatched += 1
                    continue
                site = found[0]
                chain = (site.get("chains") or [{}])[0]
                call = ("no call" if not site.get("contains_call")
                        else "contracted call" if site.get("call_contracted")
                        else "uncontracted call")
                key = tuple(json.dumps(part) if isinstance(part, (dict, list)) else str(part)
                            for part in (site.get("site"), site.get("position"), call,
                                         chain.get("member_class"), chain.get("base_cause")))
                leaves[key] += 1
                rows[key].add((unit["file"], function["function"]))
    print("| site | position | call | member class | base cause | leaves | rows |")
    print("|---|---|---|---|---|---:|---:|")
    for key, count in leaves.most_common(40):
        print("| " + " | ".join(str(part) for part in key) + f" | {count} | {len(rows[key])} |")
    print(f"\nleaves with no matching gate site (Monkey and unrecorded sites): {unmatched}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
