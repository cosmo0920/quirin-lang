#!/usr/bin/env python3
"""Attribute every moving function of a comparison by its removed leaves.

Usage: move_reasons.py COMPARE_DIR OUT_JSON

For each Unknown -> Safe (or other) move, lists every removed and added
diagnostic, its attribution (`direct`: a W47 position row of the AFTER parity
admits the same function and line) and the W47 positions involved, and
classifies the move:

* `direct`: every removed leaf is a direct W47 admission;
* `summary-cascade`: every non-direct removed leaf is an uncontracted-call
  leaf (a callee that became Safe now serves the call through the existing
  summary rule), or an unresolved alias that disappears with it;
* `mixed` or `nondirect`: anything else, reviewed by reading in the report.
"""

import collections
import gzip
import json
import pathlib
import sys


def main():
    compare, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    comparison = json.loads((compare / "comparison.json").read_text())
    leaves = collections.defaultdict(list)
    for line in gzip.open(compare / "changed-leaves.jsonl.gz", "rt"):
        row = json.loads(line)
        leaves[(row["unit"], row["function"])].append(row)
    rows = []
    tally = collections.Counter()
    for move in comparison["moves"]:
        items = leaves[(move["unit"], move["function"])]
        removed = [r for r in items if r["change"] == "removed"]
        added = [r for r in items if r["change"] == "added"]
        nondirect = [r for r in removed if not r["attribution"].startswith("direct")]
        if not nondirect:
            kind = "direct"
        elif all(r["family"] in ("unsupported:UNC", "unknown_alias") for r in nondirect):
            kind = "summary-cascade" if len(nondirect) == len(removed) else "direct+summary-cascade"
        else:
            kind = "nondirect-review"
        positions = sorted({p for r in removed for p in r["positions"]})
        tally[kind] += 1
        rows.append(dict(move, kind=kind, positions=positions,
                         removed=[{"family": r["family"], "attribution": r["attribution"],
                                   "description": r["leaf"]["description"],
                                   "origin": r["leaf"]["origin"], "line": r["leaf"]["line"]}
                                  for r in removed],
                         added=[{"family": r["family"], "description": r["leaf"]["description"],
                                 "origin": r["leaf"]["origin"]} for r in added]))
    out.write_text(json.dumps({"tally": dict(tally), "moves": rows}, indent=1, sort_keys=True) + "\n")
    print(json.dumps(dict(tally), indent=1, sort_keys=True))
    for row in rows:
        print(row["kind"], row["library"], row["file"], row["function"], row["positions"],
              [(r["family"], r["attribution"], r["origin"]) for r in row["removed"] if not r["attribution"].startswith("direct")])


if __name__ == "__main__":
    main()
