#!/usr/bin/env python3
"""prefix_size.py PREFIX REPORT... -- CG22-CALLBACK-REGISTRY-CONTRACT: for each
route report (a run directory or its resource-diagnostics.json), the leaves
`call to uncontracted function `F`` with F starting with PREFIX, the functions
carrying one, and the functions every leaf of which is such a call (decided
alone by the family)."""
import json
import pathlib
import re
import sys

UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]*)`$")


def main():
    prefix = sys.argv[1]
    for argument in sys.argv[2:]:
        path = pathlib.Path(argument)
        if path.is_dir():
            path = path / "resource-diagnostics.json"
        report = json.loads(path.read_text())
        leaves, functions, alone, callees = 0, 0, 0, set()
        for unit in report["units"]:
            for function in unit["functions"]:
                diagnostics = function.get("diagnostics") or []
                mine = [m.group(1) for m in (UNCONTRACTED.match(d["description"]) for d in diagnostics)
                        if m and m.group(1).startswith(prefix)]
                if not mine:
                    continue
                callees.update(mine)
                leaves += len(mine)
                functions += 1
                alone += len(mine) == len(diagnostics) and function["status"] == "Unknown"
        print("%s %s: leaves %d, functions %d, alone %d, callees %d" % (
            argument, prefix, leaves, functions, alone, len(callees)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
