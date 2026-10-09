#!/usr/bin/env python3
"""candidate_status.py REPORT REASONS.json -- CG22-CALLBACK-REGISTRY-CONTRACT:
for every callback candidate a `callback-uncontracted` refusal names
(reasons.py's output), its row status in REPORT, or `no row` when no analysed
unit reports a row of that name (a header `static` definition W4 does not
summarise, or a definition outside the projection). A candidate with a row
that is not `Safe` has no validated summary; one with no row has none either."""
import collections
import json
import pathlib
import sys


def main():
    path = pathlib.Path(sys.argv[1])
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    report = json.loads(path.read_text())
    status = collections.defaultdict(set)
    for unit in report["units"]:
        for function in unit["functions"]:
            status[function["function"]].add(function["status"])
    named = json.loads(pathlib.Path(sys.argv[2]).read_text())["callback-uncontracted"]["named"]
    classes = collections.defaultdict(list)
    for name in sorted(named):
        key = "/".join(sorted(status[name])) if name in status else "no row"
        classes[key].append(name)
    for key, names in sorted(classes.items()):
        print("%-12s %3d  %s" % (key, len(names), ", ".join(names)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
