#!/usr/bin/env python3
"""CG03-CALL-SUMMARY-INSTANTIATION: W47's position-row count
(`docs/reports/w47-access-path-positions/scripts/position_rows.py`), unchanged
in its keys and labels, over a stream that now also carries rows without a
line (CG10's `typed-place` decisions) and CG03's `call-instantiation` rows.
A row without the W47 keys is counted by position only and not otherwise
read.

usage: position_rows.py PARITY_JSONL[.gz]
"""

import collections
import gzip
import json
import sys

path = sys.argv[1]
opener = gzip.open if path.endswith(".gz") else open
seen = set()
counts = collections.Counter()
functions = collections.defaultdict(set)
other = collections.Counter()
with opener(path, "rt") as stream:
    for line in stream:
        row = json.loads(line)
        position = row.get("position", "statement")
        if not all(key in row for key in ("function", "line", "texts", "old_route", "new_admitted")):
            other[position] += 1
            continue
        key = (row.get("source_sha256"), row["function"], row["line"], tuple(row["texts"] or []),
               position, row["old_route"], row.get("consumer"), row.get("callee"))
        if key in seen:
            continue
        seen.add(key)
        label = f'{position}|{row["old_route"]}|typed={"admitted" if row["new_admitted"] else "refused"}'
        counts[label] += 1
        functions[label].add((row.get("source_sha256"), row["function"]))
for label in sorted(counts):
    print(f"{counts[label]:7d} rows {len(functions[label]):6d} functions  {label}")
for position, n in sorted(other.items()):
    print(f"{n:7d} rows without the W47 keys  {position}")
