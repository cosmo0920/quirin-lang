#!/usr/bin/env python3
"""CG11-POINTER-VALUED-STORES: rule 10v's own census over a parity stream.

Usage: census.py PARITY_JSONL [--json OUT]

Reads the `pointer-store` rows (one per statement consultation: the
classification of each pointer-typed store the typed statement route read)
and the `pointer-store-flow` rows (the flow's decision on each admitted
classification), deduplicated per (function, line, statement text) and per
(function, line, place, value, owner, decision) respectively, since one
statement is consulted once per analysis (several analyses per function).

Prints, and with --json writes:
  * classifications by value class and admission, in statements and functions;
  * flow decisions by (value, owner, decision), in statements and functions;
  * the functions with at least one admitted flow decision and none refused.
"""

import argparse
import collections
import json
import pathlib


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    classified = {}
    flows = set()
    with args.parity.open() as stream:
        for raw in stream:
            if '"pointer-store' not in raw:
                continue
            row = json.loads(raw)
            if row.get("position") == "pointer-store":
                for item in row["classifications"]:
                    key = (row["function"], row["line"], " ".join(row["texts"]), item["place"])
                    classified[key] = (item["value"] or "unread-place", item["admitted"])
            elif row.get("position") == "pointer-store-flow":
                flows.add((row["function"], row["line"], row["place"], row["value"],
                           row["owner"], row["decision"]))
    by_value = collections.defaultdict(lambda: [0, set()])
    for (function, _line, _text, _place), (value, admitted) in classified.items():
        entry = by_value[(value, admitted)]
        entry[0] += 1
        entry[1].add(function)
    by_flow = collections.defaultdict(lambda: [0, set()])
    decided = collections.defaultdict(set)
    for function, line, place, value, owner, decision in flows:
        entry = by_flow[(value, owner, decision)]
        entry[0] += 1
        entry[1].add(function)
        decided[(function, line, place)].add(decision)
    record = {
        "classifications": [
            {"value": value, "admitted": admitted, "statements": count,
             "functions": len(functions)}
            for (value, admitted), (count, functions) in sorted(by_value.items(),
                                                                 key=lambda item: -item[1][0])
        ],
        "flow": [
            {"value": value, "owner": owner, "decision": decision, "statements": count,
             "functions": len(functions)}
            for (value, owner, decision), (count, functions) in sorted(by_flow.items(),
                                                                        key=lambda item: -item[1][0])
        ],
        # A statement analysed more than once may be admitted in one analysis
        # (plain) and refused in another (a summary's entry analysis).
        "statements_admitted_in_every_analysis": sum(
            1 for decisions in decided.values() if decisions == {"admit"}),
        "statements_admitted_in_some_analysis": sum(
            1 for decisions in decided.values() if "admit" in decisions),
    }
    print(json.dumps(record, indent=1))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
