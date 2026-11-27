#!/usr/bin/env python3
"""CG07-GENERIC-AND-STATEMENT-EXPRESSIONS: Unknown functions whose every
diagnostic is one given unsupported description (default: the call to
`__assert_fail`), i.e. functions a no-return declaration of that callee alone
could decide (an upper bound: the rest of the body is then analysed).

Usage: sole_blockers.py DIAGNOSTICS [DESCRIPTION] [--json OUT]
"""

import argparse
import collections
import json
import pathlib


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("diagnostics", type=pathlib.Path)
    parser.add_argument("description", nargs="?", default="call to uncontracted function `__assert_fail`")
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    path = args.diagnostics / "resource-diagnostics.json" if args.diagnostics.is_dir() else args.diagnostics
    data = json.loads(path.read_text())
    sole, touched, leaves = [], 0, 0
    by_unit = collections.Counter()
    for unit in data["units"]:
        for function in unit["functions"]:
            diagnostics = function.get("diagnostics", [])
            mine = [d for d in diagnostics if d.get("description") == args.description]
            if not mine:
                continue
            touched += 1
            leaves += len(mine)
            if function["status"] == "Unknown" and len(mine) == len(diagnostics):
                sole.append(f"{unit['file']}::{function['function']}")
                by_unit[unit["file"]] += 1
    record = {
        "description": args.description,
        "leaves": leaves,
        "functions_touched": touched,
        "unknown_functions_blocked_only_by_it": len(sole),
        "by_unit": dict(by_unit.most_common()),
        "functions": sorted(sole),
    }
    print(json.dumps(record, indent=1))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
