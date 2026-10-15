#!/usr/bin/env python3
"""blockers.py BEFORE AFTER OUT.json -- HANDOFF-CANDIDATE-OWN-CALLBACKS: for
every row whose leaves moved between the two corpus runs under private/hdcr/,
its status after and its remaining leaves after; grouped by the reason key
(the last parenthesised word) of the remaining leaves, with the rows each key
blocks. Writes OUT.json and prints the grouping. A reading aid: the next
blocker of each moved row, read by hand in the report."""
import collections
import json
import pathlib
import re
import sys

PRIVATE = pathlib.Path(__file__).resolve().parents[4] / "private/hdcr"
KEY = re.compile(r"\(([a-z][a-z0-9-]+)\)\s*$")


def rows(name):
    report = json.loads((PRIVATE / name / "resource-diagnostics.json").read_text())
    out = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            out[(unit["file"], function["function"])] = function
    return out


def key(description):
    found = KEY.search(description)
    if found:
        return found.group(1)
    return re.sub(r"`[^`]*`", "_", description)[:70]


def main():
    before, after = rows(sys.argv[1]), rows(sys.argv[2])
    moved = {}
    for name, row in after.items():
        old = before.get(name)
        if old is None or old.get("diagnostics") == row.get("diagnostics"):
            continue
        moved[f"{name[0]}::{name[1]}"] = {
            "status": [old["status"], row["status"]],
            "remaining": [d.get("description", "") for d in row.get("diagnostics") or []],
        }
    grouping = collections.defaultdict(set)
    for function, entry in moved.items():
        for leaf in entry["remaining"]:
            grouping[key(leaf)].add(function)
    summary = {k: len(v) for k, v in sorted(grouping.items(), key=lambda item: -len(item[1]))}
    pathlib.Path(sys.argv[3]).write_text(
        json.dumps({"moved": moved, "rows_per_remaining_key": summary}, indent=1, sort_keys=True) + "\n")
    print(len(moved), "moved rows")
    for k, count in summary.items():
        print(f"{count:4d}  {k}")


if __name__ == "__main__":
    main()
