#!/usr/bin/env python3
"""parity_diff.py BEFORE-PARITY.jsonl AFTER-PARITY.jsonl [OUT.json] -- card
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2: the full parity streams compared
as multisets of rows (Clang node addresses erased; rows are written in
analysis order, which at --jobs 4 is not unit order), grouped by `position`:
rows only in BEFORE, only in AFTER, and the functions each group touches."""
import collections
import json
import pathlib
import re
import sys

ADDRESS = re.compile(r'"0x[0-9a-f]+"')


def load(path):
    rows = collections.Counter()
    with open(path, encoding="utf-8") as stream:
        for line in stream:
            rows[ADDRESS.sub('"0x"', line.rstrip("\n"))] += 1
    return rows


def main() -> int:
    before, after = load(sys.argv[1]), load(sys.argv[2])
    gone, new = before - after, after - before
    summary = {}
    for label, rows in (("only_before", gone), ("only_after", new)):
        by_position = collections.Counter()
        functions = collections.defaultdict(set)
        for line, count in rows.items():
            row = json.loads(line)
            position = row.get("position") or row.get("kind") or "?"
            by_position[position] += count
            functions[position].add(row.get("function"))
        summary[label] = {position: {"rows": n, "functions": len(functions[position])}
                          for position, n in by_position.most_common()}
    summary["rows"] = [sum(before.values()), sum(after.values())]
    summary["identical_rows"] = sum((before & after).values())
    print(json.dumps(summary, indent=1))
    if len(sys.argv) > 3:
        pathlib.Path(sys.argv[3]).write_text(json.dumps(summary, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
