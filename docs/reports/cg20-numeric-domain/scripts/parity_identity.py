#!/usr/bin/env python3
"""parity_identity.py BEFORE.jsonl AFTER.jsonl -- CG20-NUMERIC-DOMAIN: compare
two typed-path parity streams as multisets of rows (the stream's order follows
the analysis threads), with the run name (`/before/`, `/after/`, `-before`,
`-after`) and Clang node ids (`0x…`, addresses that vary per run) normalised. Prints the row counts and the rows only one side has."""

import collections
import re
import sys


def load(path):
    rows = collections.Counter()
    for line in open(path, encoding="utf-8"):
        line = re.sub(r"(before|after)([0-9]*)", "RUN", line.rstrip("\n"))
        rows[re.sub(r"0x[0-9a-f]+", "0xID", line)] += 1
    return rows


def main() -> int:
    before, after = load(sys.argv[1]), load(sys.argv[2])
    only_before = before - after
    only_after = after - before
    print("rows", sum(before.values()), sum(after.values()))
    print("only before", sum(only_before.values()), "only after", sum(only_after.values()))
    for row in list(only_before)[:5]:
        print("-", row[:300])
    for row in list(only_after)[:5]:
        print("+", row[:300])
    return 0 if not only_before and not only_after else 1


if __name__ == "__main__":
    sys.exit(main())
