#!/usr/bin/env python3
"""CG05-ELABORATOR-CALL-FOOTPRINTS: elaborator rows whose decision moved
between two parity streams (de-duplicated by function, line, tokens and
position).

usage: admitted.py BEFORE.jsonl AFTER.jsonl
"""

import collections
import json
import sys


def load(path):
    rows = {}
    for line in open(path):
        row = json.loads(line)
        if (row.get("position") or "").startswith("elaboration-"):
            rows[(row["function"], row["line"], tuple(row["texts"]), row["position"])] = row
    return rows


def main():
    before, after = load(sys.argv[1]), load(sys.argv[2])
    new = [k for k in after if after[k]["new_admitted"] and not (k in before and before[k]["new_admitted"])]
    lost = [k for k in before if before[k]["new_admitted"] and not (k in after and after[k]["new_admitted"])]
    changed = [
        k for k in after
        if k in before and not after[k]["new_admitted"] and not before[k]["new_admitted"]
        and after[k].get("new_refusal") != before[k].get("new_refusal")
    ]
    print("newly admitted", len(new), "functions", len({k[0] for k in new}))
    print("lost admissions", len(lost))
    print("refusal text changed", len(changed))
    print(collections.Counter((after[k]["new_refusal"] or "")[:60] for k in changed).most_common(10))
    for k in sorted(new):
        print("  ", k[0], " ".join(k[2])[:120])


if __name__ == "__main__":
    main()
