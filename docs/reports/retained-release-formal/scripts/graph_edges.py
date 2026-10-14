#!/usr/bin/env python3
"""graph_edges.py BEFORE AFTER OUT.json -- RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN:
compare two typed-path parity streams (measure.sh's NAME*-parity.jsonl) with
Clang's run-varying AST addresses and the frontend digest blanked, and list
every derivation call-graph edge (`position: summary-premises-graph`) one
stream has and the other lacks, per row (unit), plus the count of any other
row that differs after normalisation."""
import collections
import json
import re
import sys

HEX = re.compile(r'"0x[0-9a-f]+"')
FRONTEND = re.compile(r'"frontend_sha256":"[0-9a-f]+"')


def norm(line):
    return FRONTEND.sub('"frontend_sha256":""', HEX.sub('"0x"', line))


def load(path):
    with open(path, encoding="utf-8") as handle:
        return collections.Counter(norm(line) for line in handle.read().splitlines())


def edges(row):
    return {(caller, callee) for caller, callees in row["graph"].items() for callee in callees}


def main():
    before, after = load(sys.argv[1]), load(sys.argv[2])
    lost = [json.loads(line) for line in (before - after).elements()]
    gained = [json.loads(line) for line in (after - before).elements()]
    other = [row for row in lost + gained if row.get("position") != "summary-premises-graph"]
    lost_g = [row for row in lost if row.get("position") == "summary-premises-graph"]
    gained_g = [row for row in gained if row.get("position") == "summary-premises-graph"]
    pairs = []
    for old in lost_g:
        # The AFTER row of the same unit: the one whose function set is equal.
        match = next(row for row in gained_g if set(row["graph"]) == set(old["graph"]))
        gained_g.remove(match)
        pairs.append({
            "functions_in_graph": len(old["graph"]),
            "edges_added": sorted("%s -> %s" % edge for edge in edges(match) - edges(old)),
            "edges_removed": sorted("%s -> %s" % edge for edge in edges(old) - edges(match)),
        })
    out = {
        "graph_rows_changed": len(pairs),
        "graph_rows_unmatched": len(gained_g),
        "other_rows_changed": len(other),
        "rows": pairs,
    }
    with open(sys.argv[3], "w", encoding="utf-8") as handle:
        handle.write(json.dumps(out, indent=1) + "\n")
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
