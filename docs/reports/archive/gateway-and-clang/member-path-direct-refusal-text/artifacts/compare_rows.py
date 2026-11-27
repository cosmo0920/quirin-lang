#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Compare two `quirin-audit current` resource reports function row by row.

Usage: compare_rows.py BASE.json TIP.json

Prints the status tallies of each side, every row whose status moved, and
every row whose content (status, diagnostics, contract dependencies) changed,
with its diagnostic difference. `hypothesis_context` is ignored: its
`build_identity` digests the binary. Card MEMBER-PATH-DIRECT-REFUSAL-TEXT.
"""

import collections
import json
import sys


def rows(path):
    with open(path, encoding="utf-8") as handle:
        report = json.load(handle)
    out = {}
    for unit in report["units"]:
        functions = unit.get("functions") or []
        if isinstance(functions, dict):
            functions = list(functions.values())
        for index, row in enumerate(functions):
            key = (unit.get("library"), unit["file"], row.get("function"), index)
            out[key] = row
    return out


def content(row):
    return {k: v for k, v in row.items() if k != "hypothesis_context"}


def main():
    base, tip = rows(sys.argv[1]), rows(sys.argv[2])
    for name, side in (("base", base), ("tip", tip)):
        tally = collections.Counter(row.get("status") for row in side.values())
        print(f"{name}: {len(side)} rows {dict(sorted(tally.items()))}")
    unmatched = sorted(set(base) ^ set(tip), key=str)
    print(f"unmatched rows: {len(unmatched)}")
    moved = changed = 0
    for key in sorted(set(base) & set(tip), key=str):
        left, right = base[key], tip[key]
        if left.get("status") != right.get("status"):
            moved += 1
            print(f"MOVED {key}: {left.get('status')} -> {right.get('status')}")
        if content(left) != content(right):
            changed += 1
            print(f"CONTENT {key} (status {right.get('status')})")
            for field in sorted(set(content(left)) | set(content(right))):
                if left.get(field) == right.get(field):
                    continue
                if field == "diagnostics":
                    gone = [d for d in left[field] if d not in right[field]]
                    new = [d for d in right[field] if d not in left[field]]
                    for d in gone:
                        print(f"  - {json.dumps(d, sort_keys=True)}")
                    for d in new:
                        print(f"  + {json.dumps(d, sort_keys=True)}")
                else:
                    print(f"  {field}: {json.dumps(left.get(field))} -> {json.dumps(right.get(field))}")
    print(f"status moves: {moved}; content changes: {changed}")
    return 1 if unmatched else 0


if __name__ == "__main__":
    sys.exit(main())
