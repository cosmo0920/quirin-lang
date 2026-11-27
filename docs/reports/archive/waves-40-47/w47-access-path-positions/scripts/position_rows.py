#!/usr/bin/env python3
"""Count the W47 position rows of a parity stream.

Usage: position_rows.py PARITY_JSONL[.gz]

Rows are de-duplicated by (source digest, function, line, texts, position,
route), since a function may be parsed more than once (summary validation,
condition probes). Prints admitted and refused typed answers per position,
for the old-refusal rows (the fallback) and the legacy-route rows (parity).
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
with opener(path, "rt") as stream:
    for line in stream:
        row = json.loads(line)
        position = row.get("position", "statement")
        key = (row.get("source_sha256"), row["function"], row["line"], tuple(row["texts"]),
               position, row["old_route"])
        if key in seen:
            continue
        seen.add(key)
        label = f'{position}|{row["old_route"]}|typed={"admitted" if row["new_admitted"] else "refused"}'
        counts[label] += 1
        functions[label].add((row.get("source_sha256"), row["function"]))
for label in sorted(counts):
    print(f"{counts[label]:7d} rows {len(functions[label]):6d} functions  {label}")
