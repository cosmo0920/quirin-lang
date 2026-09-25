#!/usr/bin/env python3
"""Which recognised pointer actuals at a `Released` position still refuse AFTER.

Usage: residual_releases.py CALL_ARGUMENT_CENSUS.jsonl CENSUS_DIAGNOSTICS.json AFTER_DIAGNOSTICS.json OUT.json

Only sites a reported row refused in BEFORE count (the census diagnostics are
byte-identical to BEFORE; W4 re-runs of unreported bodies also emit records).
Each distinct (unit, function, line) whose statement has
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
    before = json.load(open(census_diag))
    records = [json.loads(line) for line in open(census)]
    # `unit_seq` does not index the unit list (not every unit advances it), so
    # each sequence number maps to the unit whose reported functions its
    # records name most often.
    reported = [(u["library"] + "/" + u["file"], {f["function"] for f in u["functions"]}) for u in before["units"]]
    named = collections.defaultdict(collections.Counter)
    for record in records:
        named[record["unit_seq"]][record["function"]] += 1
    unit_of = {}
    for seq, functions in named.items():
        best = max(reported, key=lambda unit: sum(n for f, n in functions.items() if f in unit[1]))
        if sum(n for f, n in functions.items() if f in best[1]):
            unit_of[seq] = best[0]

    def refused_lines(data):
        lines = set()
        for unit in data["units"]:
            key = unit["library"] + "/" + unit["file"]
            for entry in unit["functions"]:
                for d in entry["diagnostics"]:
                    if d["description"].startswith("indirect memory access"):
                        lines.add((key, entry["function"], d["line"]))
        return lines

    refused_before = refused_lines(before)
    refused = refused_lines(json.load(open(after_diag)))
    sites = {}
    for record in records:
        if record["unit_seq"] not in unit_of:
            continue
        if not any(a["access"] and a["position"] == "released" and a["recognised"] == "pointer"
                   for a in record["arguments"]):
            continue
        unit = unit_of[record["unit_seq"]]
        site = (unit, record["function"], record["line"])
        if site in refused_before:  # a reported row's refusal, not a W4 re-run
            sites[site] = record["callee"]
    still = {k: v for k, v in sites.items() if k in refused}
    result = {
        "statements_with_a_recognised_released_pointer": len(sites),
        "still_refused_after": len(still),
        "admitted_after": len(sites) - len(still),
        "still_refused_by_callee": collections.Counter(still.values()).most_common(),
        "still_refused_by_function": collections.Counter(f"{u}:{f}" for (u, f, _) in still).most_common(),
    }
    json.dump(result, open(out, "w"), indent=1)
    print(json.dumps({k: v for k, v in result.items() if not k.startswith("still_refused_by")}, indent=1))
    print(result["still_refused_by_callee"][:25])
    print(len(result["still_refused_by_function"]), result["still_refused_by_function"][:30])


if __name__ == "__main__":
    main()
