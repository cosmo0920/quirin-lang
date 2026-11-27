#!/usr/bin/env python3
"""Card LOOP-REACQUISITION-LEAK-ACCUSATION: every row that differs between two
resource-diagnostics reports (status or diagnostic multiset), keyed by unit
file and function.

Usage: row_diff.py BEFORE.json AFTER.json [--json OUT]
"""
import collections
import json
import sys


def rows(path):
    with open(path) as handle:
        report = json.load(handle)
    table = {}
    for unit in report["units"]:
        for function in unit.get("functions", []):
            key = (unit.get("file"), function["function"])
            table[key] = (
                function.get("status"),
                collections.Counter(
                    (d.get("category"), d.get("origin"), d.get("description"))
                    for d in function.get("diagnostics", [])
                ),
            )
        table[(unit.get("file"), "<unit>")] = (unit.get("status"), collections.Counter())
    return table


def main(argv):
    out = None
    if "--json" in argv:
        index = argv.index("--json")
        out = argv[index + 1]
        argv = argv[:index] + argv[index + 2:]
    before, after = rows(argv[0]), rows(argv[1])
    changes = []
    for key in sorted(set(before) | set(after)):
        b, a = before.get(key), after.get(key)
        if b == a:
            continue
        entry = {"file": key[0], "function": key[1],
                 "before": b[0] if b else "absent", "after": a[0] if a else "absent"}
        if b and a:
            entry["removed"] = sorted(map(list, (b[1] - a[1]).elements()))
            entry["added"] = sorted(map(list, (a[1] - b[1]).elements()))
        changes.append(entry)
        print(json.dumps(entry))
    print(json.dumps({"rows_before": len(before), "rows_after": len(after), "changed": len(changes)}))
    if out:
        with open(out, "w") as handle:
            json.dump(changes, handle, indent=1)
            handle.write("\n")


if __name__ == "__main__":
    main(sys.argv[1:])
