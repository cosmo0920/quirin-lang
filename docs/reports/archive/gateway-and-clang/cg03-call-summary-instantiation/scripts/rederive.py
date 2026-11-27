#!/usr/bin/env python3
"""CG03-CALL-SUMMARY-INSTANTIATION: re-derive CG00's CG03 function list on a
corpus run.

usage: rederive.py RUN_DIR [--json OUT]

For each of CG00's 39 CG03-attributed Unknown functions (its
`attribution-leaves.jsonl.gz`, entries K12/K03), print the function's status
on RUN_DIR and its remaining leaves with the three K12/K03 descriptions; then
count, over the whole run, the leaves carrying each of those descriptions.
"""

import argparse
import collections
import gzip
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
CG00 = REPO / "docs/reports/cg00-clang-gap-baseline/artifacts/attribution-leaves.jsonl.gz"

DESCRIPTIONS = {
    "unmodeled call, aggregate or indirect access in return expression": "K12 return",
    "call has an unmodeled formal address or borrowed expression": "K03",
}
SHAPE_PREFIX = "arguments do not match resource contract"


def family(description):
    if description in DESCRIPTIONS:
        return DESCRIPTIONS[description]
    if description and description.startswith(SHAPE_PREFIX):
        return "K12 shape"
    return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    data = json.loads((args.run / "resource-diagnostics.json").read_text())
    functions = {}
    totals = collections.Counter()
    for unit in data["units"]:
        for function in unit["functions"]:
            leaves = []
            for diagnostic in function.get("diagnostics", []):
                fam = family(diagnostic.get("description"))
                if fam:
                    leaves.append((fam, diagnostic.get("line")))
                    totals[fam] += 1
            functions[(unit["file"], function["function"])] = (function["status"], leaves)
    cg00 = [json.loads(line) for line in gzip.open(CG00, "rt")]
    cg00 = [row for row in cg00 if row.get("entry") in ("K12", "K03")]
    listed = sorted({(row["file"], row["function"]) for row in cg00})
    rows = []
    for key in listed:
        status, leaves = functions.get(key, ("absent", []))
        rows.append({"file": key[0], "function": key[1], "status": status, "leaves": leaves})
        print(f"{status:8} {key[0]}::{key[1]} {leaves}")
    print("totals over the run:", dict(totals))
    print("CG00 functions:", len(listed), "by status:",
          dict(collections.Counter(row["status"] for row in rows)))
    if args.json:
        args.json.write_text(json.dumps({"functions": rows, "totals": dict(totals)}, indent=1) + "\n")


if __name__ == "__main__":
    main()
