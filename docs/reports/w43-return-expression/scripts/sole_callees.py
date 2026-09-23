#!/usr/bin/env python3
"""Rank the uncontracted callees that are the only blocker of an Unknown function.

Usage: sole_callees.py ARTIFACT_DIR [TOP]

Reads `diagnostics.json.gz` from ARTIFACT_DIR. For every Unknown function whose
diagnostics carry exactly one distinct description and that description is
`call to uncontracted function `NAME``, counts NAME. A callee's count is the
number of functions a sound contract or validated summary for NAME alone could
unblock *at this layer*; it is not a promise that they become Safe, because the
contract's own effects and the function's remaining obligations still apply.
"""

import collections
import gzip
import json
import pathlib
import re
import sys


def main() -> int:
    root = pathlib.Path(sys.argv[1])
    top = int(sys.argv[2]) if len(sys.argv) > 2 else 20
    data = json.load(gzip.open(root / "diagnostics.json.gz"))
    callees = collections.Counter()
    defined = collections.defaultdict(set)
    total = 0
    for unit in data["units"]:
        names = {entry["function"] for entry in unit["functions"]}
        for entry in unit["functions"]:
            if entry.get("status") != "Unknown":
                continue
            reasons = {d["description"] for d in entry.get("diagnostics") or []}
            if len(reasons) != 1:
                continue
            match = re.fullmatch(r"call to uncontracted function `([^`]+)`", next(iter(reasons)))
            if not match:
                continue
            total += 1
            callee = match.group(1)
            callees[callee] += 1
            if callee in names:
                defined[callee].add(unit["file"])
    print(f"Unknown functions whose only blocker is one uncontracted callee: {total}")
    print(f"distinct callees: {len(callees)}")
    print(f"{'functions':>9}  callee")
    for callee, count in callees.most_common(top):
        where = " (defined in: " + ", ".join(sorted(defined[callee])) + ")" if defined[callee] else ""
        print(f"{count:9}  {callee}{where}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
