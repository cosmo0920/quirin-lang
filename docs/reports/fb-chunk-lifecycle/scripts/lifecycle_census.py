#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card FB-CHUNK-LIFECYCLE: the chunk-lifecycle rows of one report.

usage: lifecycle_census.py REPORT.json > census.json

Every function row of the units whose file ends with one of FILES (the
storage backlog and the input-chunk wrappers), with its status and its
leaves (description, line), and each unit's status."""
import json
import sys

FILES = ("plugins/in_storage_backlog/sb.c", "src/flb_input_chunk.c")


def main() -> int:
    data = json.load(open(sys.argv[1]))
    out = []
    for unit in data["units"]:
        if not unit["file"].endswith(FILES):
            continue
        out.append({
            "unit": unit["file"],
            "status": unit.get("status"),
            "functions": [
                {
                    "function": function["function"],
                    "status": function["status"],
                    "leaves": sorted(
                        [d["description"], d["line"]] for d in function.get("diagnostics", [])
                    ),
                }
                for function in unit.get("functions", [])
            ],
        })
    json.dump(out, sys.stdout, indent=1, sort_keys=True)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
