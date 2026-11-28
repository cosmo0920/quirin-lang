#!/usr/bin/env python3
"""rows.py BASE.json TIP.json -- K2-BUILTIN-BIT-COUNTS: the status, leaves
and contract dependencies of every row that spells a bit-count builtin in the
opt-in current reports (snappy.c and lz4.c), and of log2_floor's caller."""
import json
import sys

ROWS = ("log2_floor", "find_lsb_set_non_zero", "find_lsb_set_non_zero64",
        "LZ4_NbCommonBytes", "compress_fragment")


def read(path):
    out = {}
    for unit in json.load(open(path))["units"]:
        for function in unit.get("functions", []):
            if function["function"] in ROWS:
                out[f"{unit['file']}::{function['function']}"] = {
                    "status": function["status"],
                    "leaves": sorted(f"{d.get('line')}: {d.get('description') or d.get('kind')}"
                                     for d in function.get("diagnostics", [])),
                    "contract_dependencies": function.get("contract_dependencies"),
                }
    return out


base, tip = read(sys.argv[1]), read(sys.argv[2])
json.dump({key: {"base": base.get(key), "tip": tip.get(key)}
           for key in sorted(set(base) | set(tip))}, sys.stdout, indent=1)
print()
