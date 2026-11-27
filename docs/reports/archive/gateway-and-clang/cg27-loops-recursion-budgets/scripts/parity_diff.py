#!/usr/bin/env python3
"""CG27-LOOPS-RECURSION-BUDGETS: compare two typed-path parity streams
(QCORE_TYPED_PATH_PARITY) as multisets of rows, and summarize the rows only
one side has by their keys (position, reason) so a difference is attributed.

usage: parity_diff.py BEFORE.jsonl AFTER.jsonl

Clang AST node addresses (`"decl": "0x…"`, `"field": "0x…"`) differ between
any two runs; they are normalised to `0x` before the rows are compared.
"""

import collections
import json
import re
import sys

ADDRESS = re.compile(r'"0x[0-9a-f]+"')


def rows(path):
    with open(path, encoding="utf-8") as handle:
        return collections.Counter(ADDRESS.sub('"0x"', line.rstrip("\n")) for line in handle)


def key(line):
    try:
        row = json.loads(line)
    except json.JSONDecodeError:
        return ("<unparsed>",)
    return tuple(str(row.get(field)) for field in ("position", "outcome", "reason", "function"))[:4]


def main() -> int:
    before, after = rows(sys.argv[1]), rows(sys.argv[2])
    only_before = before - after
    only_after = after - before
    print(json.dumps({
        "rows_before": sum(before.values()),
        "rows_after": sum(after.values()),
        "only_before": sum(only_before.values()),
        "only_after": sum(only_after.values()),
    }))
    for label, side in (("before", only_before), ("after", only_after)):
        summary = collections.Counter()
        for line, count in side.items():
            summary[key(line)] += count
        for item, count in summary.most_common(20):
            print(label, count, item)
    for line in list(only_before)[:2] + list(only_after)[:2]:
        print(line[:600])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
