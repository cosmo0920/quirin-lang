#!/usr/bin/env python3
"""parity_rows.py BEFORE.jsonl AFTER.jsonl FUNCTION [N] -- REFCOUNT-RESOURCE-KIND (adapted from SLOT-MOVE-AND-INTERIOR-BORROW, from W43-ABI-ENUM-AND-CONST-PP, from CALLEE-RETENTION-CLASS1):
print up to N (default 4) rows of FUNCTION that only one of two parity streams
has (Clang AST node addresses erased), so a difference outside a moving
function can be read."""
import collections
import json
import re
import sys

ADDRESS = re.compile(r'"0x[0-9a-f]+"')


def rows(path, function):
    out = collections.Counter()
    with open(path) as handle:
        for line in handle:
            if function not in line:
                continue
            erased = ADDRESS.sub('"0x"', line.rstrip("\n"))
            try:
                row = json.loads(erased)
            except json.JSONDecodeError:
                continue
            if row.get("function") == function:
                out[erased] += 1
    return out


def main():
    before = rows(sys.argv[1], sys.argv[3])
    after = rows(sys.argv[2], sys.argv[3])
    limit = int(sys.argv[4]) if len(sys.argv) > 4 else 4
    only_before = list((before - after).elements())[:limit]
    only_after = list((after - before).elements())[:limit]
    for label, items in (("BEFORE", only_before), ("AFTER", only_after)):
        for item in items:
            print(label, item[:1500])


if __name__ == "__main__":
    main()
