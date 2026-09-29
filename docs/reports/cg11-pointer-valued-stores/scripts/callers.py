#!/usr/bin/env python3
"""CG11-POINTER-VALUED-STORES: the callers a withheld summary leaves refused.

Usage: callers.py RUN_DIR FUNCTION...

Counts, in RUN_DIR's resource-diagnostics.json, the `call to uncontracted
function NAME` leaves naming each FUNCTION (a body rule 10v admitted, which
publishes no summary), and the calling functions with their status.
"""

import collections
import json
import pathlib
import re
import sys

PATTERN = re.compile(r"call to uncontracted function `([^`]+)`")


def main() -> int:
    run = pathlib.Path(sys.argv[1])
    wanted = set(sys.argv[2:])
    data = json.loads((run / "resource-diagnostics.json").read_text())
    sites = collections.Counter()
    callers = collections.defaultdict(set)
    for unit in data["units"]:
        for function in unit["functions"]:
            for diagnostic in function.get("diagnostics", []):
                match = PATTERN.match(diagnostic.get("description") or "")
                if match and match.group(1) in wanted:
                    sites[match.group(1)] += 1
                    callers[match.group(1)].add(
                        (unit["file"], function["function"], function["status"]))
    for name in sorted(wanted):
        print(f"{name}: {sites[name]} leaves, {len(callers[name])} callers")
        for caller in sorted(callers[name]):
            print(f"    {caller[0]}::{caller[1]} {caller[2]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
