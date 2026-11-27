#!/usr/bin/env python3
"""deps.py A B [FUNCTION...] -- HANDOFF-CANDIDATE-OWN-CALLBACKS: between two
corpus runs under private/uorw/ (A and B), the rows whose
`contract_dependencies` differ (by status), the dependency counts and Safe
roll-up of each sidecar, the fold's demotions, and the dependencies of each
named FUNCTION in B. A reading aid for the report."""
import collections
import json
import pathlib
import sys

PRIVATE = pathlib.Path(__file__).resolve().parents[4] / "private/uorw"


def rows(name):
    report = json.loads((PRIVATE / name / "resource-diagnostics.json").read_text())
    out = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            out[(unit.get("library", ""), unit["file"], function["function"])] = function
    return out


def sidecar(name):
    return json.loads((PRIVATE / name / "resource-diagnostics.contracts.json").read_text())


def main():
    first, second, *named = sys.argv[1:]
    a, b = rows(first), rows(second)
    changed = [key for key in b if b[key].get("contract_dependencies") != a.get(key, {}).get("contract_dependencies")]
    moved = [key for key in b if key in a and (b[key]["status"] != a[key]["status"]
                                               or b[key].get("diagnostics") != a[key].get("diagnostics"))]
    print("rows whose dependencies differ:", len(changed),
          dict(collections.Counter(b[key]["status"] for key in changed)))
    print("rows whose status or leaves differ:", len(moved))
    for name in (first, second):
        dependencies = sidecar(name)["contract_dependencies"]
        rollup = {k: v for k, v in dependencies["safe_rollup"].items() if not isinstance(v, list)}
        print(name, json.dumps(dependencies["counts"]), json.dumps(rollup))
        print(name, "fold demoted:", sidecar(name)["callback_members"]["demoted"])
    for key, row in b.items():
        if key[2] in named:
            print(key, row["status"], json.dumps(row.get("contract_dependencies")))


if __name__ == "__main__":
    main()
