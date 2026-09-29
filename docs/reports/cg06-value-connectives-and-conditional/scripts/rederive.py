#!/usr/bin/env python3
"""CG06-VALUE-CONNECTIVES-AND-CONDITIONAL: re-derive CG00's CG06 function list
on a corpus run.

usage: rederive.py RUN_DIR [--json OUT]

For each Unknown function CG00 attributed to E06/E07 (its
`attribution-leaves.jsonl.gz`), print the function's status on RUN_DIR, how
many of CG00's E06/E07 leaves (keyed by internal line and description) are
still present, and the function's other leaves by description. The 40
functions CG00 predicted freed alone (`predictions/CG06.md`) are marked `*`.
"""

import argparse
import collections
import gzip
import json
import pathlib
import re

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
CG00 = REPO / "docs/reports/cg00-clang-gap-baseline/artifacts/attribution-leaves.jsonl.gz"
PREDICTION = REPO / "docs/reports/cg00-clang-gap-baseline/predictions/CG06.md"


def load(run):
    path = run / "resource-diagnostics.json"
    data = json.loads(path.read_text())
    functions = {}
    for unit in data["units"]:
        for function in unit["functions"]:
            leaves = collections.Counter()
            for diagnostic in function.get("diagnostics", []):
                description = diagnostic.get("description")
                if diagnostic.get("category") == "unknown_alias":
                    description = "unresolved resource identity"
                leaves[(diagnostic.get("line"), description)] += 1
            functions[(unit["file"], function["function"])] = (function["status"], leaves)
    return functions


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    functions = load(args.run)
    cg00 = [json.loads(line) for line in gzip.open(CG00, "rt")]
    cg00 = [row for row in cg00 if row.get("entry") in ("E06", "E07")]
    by_function = collections.defaultdict(list)
    for row in cg00:
        by_function[(row["file"], row["function"])].append((row["line"], row["description"]))
    predicted = set(re.findall(r"^- `([^`]+)::([^`]+)` — ", PREDICTION.read_text(), re.M))
    rows = []
    for key in sorted(by_function):
        status, leaves = functions.get(key, ("absent", collections.Counter()))
        attributed = by_function[key]
        present = sum(1 for leaf in attributed if leaves.get(leaf))
        others = collections.Counter()
        for (line, description), count in leaves.items():
            if (line, description) not in attributed:
                others[description] += count
        rows.append({
            "file": key[0],
            "function": key[1],
            "predicted_alone": key in predicted,
            "status": status,
            "cg00_leaves": len(attributed),
            "cg00_leaves_present": present,
            "other_leaves": dict(others),
        })
    for row in rows:
        if row["predicted_alone"]:
            print(f"* {row['status']:8} {row['file']}::{row['function']} "
                  f"cg00 {row['cg00_leaves_present']}/{row['cg00_leaves']} others {row['other_leaves']}")
    print("CG00 E06/E07 functions:", len(rows), "by status:",
          dict(collections.Counter(row["status"] for row in rows)))
    alone = [row for row in rows if row["predicted_alone"]]
    print("predicted alone:", len(alone), "by status:",
          dict(collections.Counter(row["status"] for row in alone)))
    print("CG00 leaves present:", sum(r["cg00_leaves_present"] for r in rows),
          "of", sum(r["cg00_leaves"] for r in rows))
    if args.json:
        args.json.write_text(json.dumps({"functions": rows}, indent=1) + "\n")


if __name__ == "__main__":
    main()
