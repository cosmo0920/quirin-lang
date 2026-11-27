#!/usr/bin/env python3
"""CG12-VOID-VIEW-READBACK: the leaves left at CG00's 76 M05 origins in a run,
by category and description, with the functions carrying them.

usage: residual_origins.py RUN_DIR
"""

import collections
import gzip
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
CG00 = HERE.parents[1] / "cg00-clang-gap-baseline/artifacts/attribution-leaves.jsonl.gz"


def main() -> int:
    rows = [json.loads(line) for line in gzip.open(CG00)]
    m05 = {(r["file"], r["function"], r["origin"]) for r in rows if r.get("entry") == "M05"}
    data = json.loads((pathlib.Path(sys.argv[1]) / "resource-diagnostics.json").read_bytes())
    counts = collections.Counter()
    functions = collections.defaultdict(set)
    for unit in data["units"]:
        for function in unit["functions"]:
            for leaf in function.get("diagnostics", []):
                if (unit["file"], function["function"], leaf.get("origin")) not in m05:
                    continue
                key = (leaf["category"], (leaf.get("description") or "").split("`")[0])
                counts[key] += 1
                functions[key].add(function["function"])
    for key, count in counts.most_common():
        print(json.dumps({"leaves": count, "category": key[0], "description": key[1],
                          "functions": sorted(functions[key])}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
