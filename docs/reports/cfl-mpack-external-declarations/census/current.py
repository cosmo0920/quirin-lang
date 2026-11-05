#!/usr/bin/env python3
"""CFL-MPACK-EXTERNAL-DECLARATIONS: the user's `current` run, before and predicted.

    python3 current.py REPORT DECLARED.txt

Prints the run's uncontracted callees and pairs, the "blocked only by
uncontracted calls" set, the in-scope rows, and the declared names' share.
"""
import collections
import json
import re
import sys
from pathlib import Path

UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]+)`$")


def main():
    report = json.loads(Path(sys.argv[1]).read_text())
    declared = {line.strip() for line in Path(sys.argv[2]).read_text().splitlines()
                if line.strip() and not line.startswith("#")}
    pairs = collections.defaultdict(set)
    only = []
    units = collections.Counter()
    for unit in report["units"]:
        units[unit["status"]] += 1
        for function in unit["functions"]:
            names = [UNCONTRACTED.match(d["description"]) for d in function["diagnostics"]]
            for m in names:
                if m:
                    pairs[m.group(1)].add((unit["file"], function["function"]))
            if function["diagnostics"] and all(names):
                only.append((unit["file"], function["function"], {m.group(1) for m in names}))
    in_scope = {n for n in pairs if n.startswith(("cfl_", "mpack_"))}
    leaving = sorted(n for n in in_scope if n in declared)
    freed = [(f, fn) for f, fn, callees in only if callees <= declared]
    print("units:", dict(units))
    print("uncontracted callees:", len(pairs), "pairs:", sum(len(v) for v in pairs.values()))
    print("in-scope (cfl_/mpack_) callees:", len(in_scope),
          "pairs:", sum(len(pairs[n]) for n in in_scope))
    print("declared callees present in the run:", len(leaving),
          "pairs:", sum(len(pairs[n]) for n in leaving))
    print("only-uncontracted functions:", len(only))
    print("of them, every uncontracted callee declared here:", len(freed))
    for f, fn in freed:
        print("  ", f, fn)
    print("declared but absent from the run:", sorted(declared - set(pairs)))


if __name__ == "__main__":
    main()
