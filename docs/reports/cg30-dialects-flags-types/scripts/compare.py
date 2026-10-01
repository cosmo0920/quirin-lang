#!/usr/bin/env python3
"""CG30-DIALECTS-FLAGS-TYPES: compare two resource-diagnostics.json reports.

usage: compare.py BEFORE.json AFTER.json [OUT.json]

Prints status totals (keyed by file and function, as the earlier cards count
them) and SHA-256 of both, every function whose status moved, and every
function whose diagnostic multiset changed (added/removed leaves, by
description). `normalised_sha256` hashes the bytes with the BEFORE tree's
path (`private/cg30dft/base-tree/`, embedded in provider paths) removed, so a
BEFORE from the exported base tree and an AFTER from the worktree compare
byte for byte. Read-only.
"""

import collections
import hashlib
import json
import pathlib
import sys


def rows(path):
    report = json.loads(pathlib.Path(path).read_text())
    table = {}
    for unit in report["units"]:
        for function in unit.get("functions", []):
            key = f"{unit.get('file')}::{function['function']}"
            leaves = collections.Counter(
                f"{d.get('category')}: {d.get('description')}" for d in function.get("diagnostics", [])
            )
            table[key] = (function["status"], leaves)
    return table


def normalised(path):
    data = pathlib.Path(path).read_bytes().replace(b"private/cg30dft/base-tree/", b"")
    return hashlib.sha256(data).hexdigest()


def totals(table):
    return dict(collections.Counter(status for status, _ in table.values()))


def main():
    before_path, after_path = sys.argv[1], sys.argv[2]
    before, after = rows(before_path), rows(after_path)
    result = {
        "before": {
            "sha256": hashlib.sha256(pathlib.Path(before_path).read_bytes()).hexdigest(),
            "normalised_sha256": normalised(before_path),
            "totals": totals(before),
        },
        "after": {
            "sha256": hashlib.sha256(pathlib.Path(after_path).read_bytes()).hexdigest(),
            "normalised_sha256": normalised(after_path),
            "totals": totals(after),
        },
        "only_before": sorted(set(before) - set(after)),
        "only_after": sorted(set(after) - set(before)),
        "moved": [],
        "leaves": [],
    }
    for key in sorted(set(before) & set(after)):
        (b_status, b_leaves), (a_status, a_leaves) = before[key], after[key]
        if b_status != a_status:
            result["moved"].append({"function": key, "before": b_status, "after": a_status})
        if b_leaves != a_leaves:
            result["leaves"].append(
                {
                    "function": key,
                    "status": [b_status, a_status],
                    "added": sorted((a_leaves - b_leaves).elements()),
                    "removed": sorted((b_leaves - a_leaves).elements()),
                }
            )
    text = json.dumps(result, indent=1, ensure_ascii=False)
    if len(sys.argv) > 3:
        pathlib.Path(sys.argv[3]).write_text(text + "\n")
    print(text)


if __name__ == "__main__":
    main()
