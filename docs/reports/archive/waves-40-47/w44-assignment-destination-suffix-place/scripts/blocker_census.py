#!/usr/bin/env python3
"""Blocker census of one audit run: leaves, Unknown functions and sole-blocked
functions per refusal reason.

Usage: blocker_census.py DIAGNOSTICS_JSON [TOP]

A reason is a diagnostic description with every backtick-quoted span replaced
by `_`. The population is the diagnostics of **Unknown** functions; a function
is *sole-blocked* by a reason when it is that function's only distinct reason.
Written for card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE to re-derive the row
published in docs/reports/w44-indirect-chain-access/blocker-census.txt; run
over that card's baseline it reproduces the row (17,236 / 3,278 / 408 for the
indirect-access refusal).
"""

import collections
import gzip
import json
import pathlib
import re
import sys


def read(path):
    data = pathlib.Path(path).read_bytes()
    if path.endswith(".gz"):
        data = gzip.decompress(data)
    return json.loads(data)


def reason(description):
    return re.sub(r"`[^`]*`", "`_`", description or "")


def main():
    document = read(sys.argv[1])
    top = int(sys.argv[2]) if len(sys.argv) > 2 else 20
    leaves = collections.Counter()
    functions = collections.Counter()
    sole = collections.Counter()
    unknown = 0
    for unit in document["units"]:
        for function in unit["functions"]:
            if function["status"] != "Unknown":
                continue
            unknown += 1
            reasons = set()
            for diagnostic in function["diagnostics"]:
                text = reason(diagnostic.get("description"))
                leaves[text] += 1
                reasons.add(text)
            for text in reasons:
                functions[text] += 1
            if len(reasons) == 1:
                sole[next(iter(reasons))] += 1
    print(f"Unknown functions: {unknown}")
    print("  leaves  functions   sole  reason")
    for text, count in leaves.most_common(top):
        print(f"{count:>8,} {functions[text]:>10,} {sole[text]:>6,}  {text}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
