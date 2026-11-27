#!/usr/bin/env python3
"""CG21-CALL-EXITS-AND-BOUNDARIES (copied from CG05-ELABORATOR-CALL-FOOTPRINTS, from CG05-NESTED-CALL-EFFECTS, from CG03-CALL-SUMMARY-INSTANTIATION): every function whose diagnostics changed
between two runs, with its status move and leaf delta.

usage: changed.py BEFORE AFTER [--json OUT]   (run directories or diagnostics files)
"""

import argparse
import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from compare import load  # noqa: E402


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("before", type=pathlib.Path)
    parser.add_argument("after", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    _, before = load(args.before)
    _, after = load(args.after)
    rows = []
    for key in sorted(set(before) | set(after), key=str):
        b_status, b_leaves = before.get(key, ("absent", collections.Counter()))
        a_status, a_leaves = after.get(key, ("absent", collections.Counter()))
        if b_status == a_status and b_leaves == a_leaves:
            continue
        delta = collections.Counter(a_leaves)
        delta.subtract(b_leaves)
        rows.append({
            "unit": key[0],
            "function": key[1],
            "before": b_status,
            "after": a_status,
            "leaf_delta": {f"{c}: {d}": n for (c, d), n in sorted(delta.items(), key=str) if n},
        })
    for row in rows:
        print(f"{row['before']:8} -> {row['after']:8} {row['unit']}::{row['function']}")
        for leaf, n in row["leaf_delta"].items():
            print(f"      {n:+d} {leaf}")
    print(len(rows), "functions changed")
    if args.json:
        args.json.write_text(json.dumps(rows, indent=1) + "\n")


if __name__ == "__main__":
    main()
