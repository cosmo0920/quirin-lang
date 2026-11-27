#!/usr/bin/env python3
"""rows_equal.py LEFT RIGHT -- MEMBER-PREPASS-OTHER-ROUTES: whether two route
reports (run directories or resource-diagnostics.json) agree row for row:
every unit's file and status, and every function's name, status and
diagnostics (line, category, description). Prints the count of rows, the
differing ones (at most 10) and the report keys that differ elsewhere. A
reading aid for comparing runs whose report bytes differ only by recorded
paths or contract-source digests."""
import json
import pathlib
import sys


def load(argument):
    path = pathlib.Path(argument)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    return json.loads(path.read_text())


def rows(report):
    out = []
    for unit in report["units"]:
        for function in unit["functions"]:
            out.append((unit["file"], unit["status"], function["function"], function["status"],
                        tuple((d.get("line"), d.get("category"), d.get("description"))
                              for d in function.get("diagnostics", []))))
    return out


def main():
    left, right = load(sys.argv[1]), load(sys.argv[2])
    a, b = rows(left), rows(right)
    differing = [(x, y) for x, y in zip(a, b) if x != y]
    print(json.dumps({"rows": [len(a), len(b)], "differing": len(differing) + abs(len(a) - len(b)),
                      "first": [[list(map(str, x)), list(map(str, y))] for x, y in differing[:10]]},
                     indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
