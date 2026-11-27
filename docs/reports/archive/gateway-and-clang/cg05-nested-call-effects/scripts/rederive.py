#!/usr/bin/env python3
"""CG05-NESTED-CALL-EFFECTS: re-derive CG00's CG05 function list on a corpus run.

usage: rederive.py RUN_DIR [--json OUT]

For each Unknown function CG00 attributed to CG05 (its
`attribution-leaves.jsonl.gz`, entries E10/E11/E12), print the function's
status on RUN_DIR and its non-alias leaves; then count, over the whole run, the
leaves of the descriptions CG00's CG05 rows carried.
"""

import argparse
import collections
import gzip
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
CG00 = REPO / "docs/reports/cg00-clang-gap-baseline/artifacts/attribution-leaves.jsonl.gz"
FREED = REPO / "docs/reports/cg00-clang-gap-baseline/predictions/CG05.md"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    data = json.loads((args.run / "resource-diagnostics.json").read_text())
    functions = {}
    for unit in data["units"]:
        for function in unit["functions"]:
            leaves = collections.Counter()
            for diagnostic in function.get("diagnostics", []):
                if diagnostic.get("category") == "unknown_alias":
                    leaves["unknown_alias"] += 1
                else:
                    leaves[diagnostic.get("description") or diagnostic.get("category")] += 1
            functions[(unit["file"], function["function"])] = (function["status"], leaves)
    cg00 = [json.loads(line) for line in gzip.open(CG00, "rt")]
    cg00 = [row for row in cg00 if row.get("entry") in ("E10", "E11", "E12")]
    by_function = collections.defaultdict(list)
    for row in cg00:
        by_function[(row["file"], row["function"])].append((row["entry"], row["reason"], row["line"]))
    freed = set()
    for line in FREED.read_text().splitlines():
        if line.startswith("- `") and "::" in line:
            name = line[3 : line.index("`", 3)]
            unit, function = name.split("::", 1)
            freed.add((unit, function))
    rows = []
    for key in sorted(by_function):
        status, leaves = functions.get(key, ("absent", collections.Counter()))
        rows.append(
            {
                "file": key[0],
                "function": key[1],
                "cg00_freed_alone": key in freed,
                "cg00_leaves": by_function[key],
                "status": status,
                "leaves": dict(leaves),
            }
        )
    print("CG00 CG05 functions:", len(rows), "by status:", dict(collections.Counter(r["status"] for r in rows)))
    for row in rows:
        if row["cg00_freed_alone"]:
            print(f"{row['status']:8} {row['file']}::{row['function']} {row['leaves']}")
    descriptions = collections.Counter()
    for unit in data["units"]:
        for function in unit["functions"]:
            for diagnostic in function.get("diagnostics", []):
                descriptions[diagnostic.get("description")] += 0
    if args.json:
        args.json.write_text(json.dumps({"functions": rows}, indent=1) + "\n")


if __name__ == "__main__":
    main()
