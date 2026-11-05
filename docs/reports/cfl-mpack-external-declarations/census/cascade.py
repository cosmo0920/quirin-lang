#!/usr/bin/env python3
"""CFL-MPACK-EXTERNAL-DECLARATIONS: the rows a set of new declarations could free.

    python3 cascade.py REPORT DECLARED.txt

DECLARED.txt lists the callee names the card declares, one per line. A row is
a candidate when every leaf it carries is `call to uncontracted function X`
with X declared, or X a function of the same library that is itself a
candidate (the W4 cascade: a freed body may publish a summary its callers
read). This is an upper bound on Unknown -> Safe moves, not a prediction of
the analyser: a declaration can expose a leaf the uncontracted call masked.
"""
import json
import re
import sys
from pathlib import Path

UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]+)`$")


def main():
    report = json.loads(Path(sys.argv[1]).read_text())
    declared = {line.strip() for line in Path(sys.argv[2]).read_text().splitlines()
                if line.strip() and not line.startswith("#")}
    rows = []
    for unit in report["units"]:
        for function in unit["functions"]:
            rows.append((unit["library"], unit["file"], function["function"], function["status"],
                         [d["description"] for d in function["diagnostics"]]))
    freed = set()
    rounds = []
    while True:
        new = []
        for library, file, name, status, leaves in rows:
            key = (library, file, name)
            if key in freed or status == "Safe" or not leaves:
                continue
            ok = True
            touched = False
            for leaf in leaves:
                m = UNCONTRACTED.match(leaf)
                if not m:
                    ok = False
                    break
                callee = m.group(1)
                if callee in declared:
                    touched = True
                    continue
                if any(f[0] == library and f[2] == callee for f in freed):
                    touched = True
                    continue
                ok = False
                break
            if ok and touched:
                new.append(key)
        if not new:
            break
        freed.update(new)
        rounds.append(sorted(new))
    for i, keys in enumerate(rounds, 1):
        print(f"round {i}: {len(keys)}")
        for library, file, name in keys:
            print(f"  {library}::{file}::{name}")


if __name__ == "__main__":
    main()
