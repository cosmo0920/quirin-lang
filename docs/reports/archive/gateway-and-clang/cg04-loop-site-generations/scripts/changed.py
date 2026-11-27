#!/usr/bin/env python3
"""CG04-LOOP-SITE-GENERATIONS: every function whose row changed between two runs.

Usage: changed.py BEFORE AFTER [--json OUT]   (run directories or report files)

Prints, per (unit, function) whose status or diagnostic multiset changed, the
status move and the diagnostic delta (category and description, with lines).
"""

import argparse
import collections
import json
import pathlib


def load(path: pathlib.Path):
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    rows = {}
    for unit in data["units"]:
        for function in unit["functions"]:
            rows[(unit["file"], function["function"])] = (
                function["status"],
                collections.Counter(
                    json.dumps(d, sort_keys=True) for d in function.get("diagnostics", [])
                ),
            )
    return rows


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("before", type=pathlib.Path)
    parser.add_argument("after", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    before, after = load(args.before), load(args.after)
    record = []
    for key in sorted(set(before) | set(after)):
        b_status, b_diags = before.get(key, ("absent", collections.Counter()))
        a_status, a_diags = after.get(key, ("absent", collections.Counter()))
        if (b_status, b_diags) == (a_status, a_diags):
            continue
        record.append({
            "unit": key[0],
            "function": key[1],
            "before": b_status,
            "after": a_status,
            "added": sorted((a_diags - b_diags).elements()),
            "removed": sorted((b_diags - a_diags).elements()),
        })
    for entry in record:
        print(f"{entry['unit']}::{entry['function']} {entry['before']} -> {entry['after']}")
        for sign, rows in (("+", entry["added"]), ("-", entry["removed"])):
            for row in rows:
                d = json.loads(row)
                print(f"   {sign} {d.get('category')} {d.get('description', '')[:90]} @{d.get('origin', '')}")
    print(f"{len(record)} functions changed")
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
