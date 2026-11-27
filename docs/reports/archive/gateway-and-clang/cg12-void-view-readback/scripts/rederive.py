#!/usr/bin/env python3
"""CG12-VOID-VIEW-READBACK: re-derive CG00's M05 function list on a run.

usage: rederive.py RUN_DIR [--json OUT]

For each of the 33 functions CG00 attributed M05 leaves to (its
`attribution-leaves.jsonl.gz`, entry M05), print the function's status in
RUN_DIR's `resource-diagnostics.json`, how many of CG00's M05 origins still
carry a leaf, and every other leaf (description, origin) the function has.
"""

import collections
import gzip
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
CG00 = HERE.parents[1] / "cg00-clang-gap-baseline/artifacts/attribution-leaves.jsonl.gz"


def leaf_key(diagnostic):
    description = diagnostic.get("description")
    if diagnostic.get("category") == "unknown_alias":
        description = "unresolved resource identity"
    return diagnostic.get("category"), description, diagnostic.get("origin")


def main() -> int:
    run = pathlib.Path(sys.argv[1])
    rows = [json.loads(line) for line in gzip.open(CG00)]
    m05 = [row for row in rows if row.get("entry") == "M05"]
    origins = collections.defaultdict(set)
    for row in m05:
        origins[(row["file"], row["function"])].add(row["origin"])
    data = json.loads((run / "resource-diagnostics.json").read_bytes())
    functions = {}
    for unit in data["units"]:
        for function in unit["functions"]:
            functions[(unit["file"], function["function"])] = function
    record = []
    for key in sorted(origins):
        function = functions.get(key)
        if function is None:
            record.append({"file": key[0], "function": key[1], "status": "absent"})
            continue
        leaves = collections.Counter(leaf_key(d) for d in function.get("diagnostics", []))
        m05_left = sum(count for (_, _, origin), count in leaves.items() if origin in origins[key])
        others = sorted(
            f"{count}x {category}: {description} @ {origin}"
            for (category, description, origin), count in leaves.items()
            if origin not in origins[key]
        )
        record.append({
            "file": key[0],
            "function": key[1],
            "status": function["status"],
            "cg00_m05_origins": len(origins[key]),
            "leaves_at_m05_origins": m05_left,
            "other_leaves": others,
        })
    for row in record:
        print(json.dumps(row))
    if len(sys.argv) > 3 and sys.argv[2] == "--json":
        pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
