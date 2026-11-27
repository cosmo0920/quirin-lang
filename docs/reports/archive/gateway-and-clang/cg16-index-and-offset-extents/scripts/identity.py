#!/usr/bin/env python3
"""identity.py NAME_A NAME_B -- CG16-INDEX-AND-OFFSET-EXTENTS: the custody
artifacts of two runs of scripts/measure.sh on the three routes (corpus,
system, tail-pause slice): SHA-256 of each resource-diagnostics.json, the
de-duplicated function totals and whether the bytes are identical.
"""
import collections
import hashlib
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/cg16ix"
SLICES = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")


def routes(name):
    return {
        "corpus": PRIVATE / name,
        "system": PRIVATE / f"{name}-system",
        "slice": SLICES / f"cg16ix-{name}",
    }


def totals(path):
    report = json.loads(path.read_text())
    statuses = {}
    for unit in report["units"]:
        for function in unit.get("functions", []):
            statuses.setdefault((unit["file"], function["function"]), set()).add(function["status"])
    count = collections.Counter()
    for found in statuses.values():
        for status in ("Unsafe", "Unknown", "Pending", "Safe"):
            if status in found:
                count[status] += 1
                break
    return dict(sorted(count.items()))


def main():
    a, b = sys.argv[1], sys.argv[2]
    record = {}
    for route, before in routes(a).items():
        after = routes(b)[route]
        first = (before / "resource-diagnostics.json").read_bytes()
        second = (after / "resource-diagnostics.json").read_bytes()
        record[route] = {
            "before": {"sha256": hashlib.sha256(first).hexdigest(), "totals": totals(before / "resource-diagnostics.json")},
            "after": {"sha256": hashlib.sha256(second).hexdigest(), "totals": totals(after / "resource-diagnostics.json")},
            "byte_identical": first == second,
        }
    json.dump(record, sys.stdout, indent=1)
    print()


if __name__ == "__main__":
    main()
