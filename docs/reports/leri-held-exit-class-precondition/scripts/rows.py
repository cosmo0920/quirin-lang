#!/usr/bin/env python3
"""rows.py REPORT FUNCTION... -- the named functions' rows of one `current`
report: unit, function, status and every leaf (line and description)."""
import json
import sys


def main() -> int:
    data = json.load(open(sys.argv[1]))
    wanted = set(sys.argv[2:])
    for unit in data["units"]:
        for function in unit["functions"]:
            if function["function"] not in wanted:
                continue
            print(unit["file"].split("/")[-1], function["function"], function["status"])
            for leaf in function["diagnostics"]:
                print("   ", leaf.get("line"), leaf["description"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
