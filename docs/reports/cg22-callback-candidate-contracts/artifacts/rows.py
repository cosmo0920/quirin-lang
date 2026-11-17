#!/usr/bin/env python3
"""rows.py REPORT PATTERN... -- CG22-CALLBACK-CANDIDATE-CONTRACTS: the rows
whose function name is one of PATTERN, or any of whose leaves mentions one,
with their status and every leaf description."""
import json
import pathlib
import sys


def main():
    path = pathlib.Path(sys.argv[1])
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    report = json.loads(path.read_text())
    patterns = sys.argv[2:]
    for unit in report["units"]:
        for function in unit["functions"]:
            leaves = [d["description"] for d in function.get("diagnostics") or []]
            if function["function"] in patterns or any(p in d for p in patterns for d in leaves):
                print("%s::%s %s" % (unit["file"], function["function"], function["status"]))
                for d in leaves:
                    print("    " + d[:300])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
