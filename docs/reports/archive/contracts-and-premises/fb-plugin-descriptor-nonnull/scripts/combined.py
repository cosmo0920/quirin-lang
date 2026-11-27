#!/usr/bin/env python3
"""combined.py BEFORE.jsonl AFTER.jsonl -- FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE:
every row whose combined (custody joined with null) verdict moved, keyed as
compare.py keys rows."""
import collections
import json
import sys


def load(path):
    rows, seen = {}, collections.Counter()
    for line in open(path):
        row = json.loads(line)
        base = (row["file"], row["function"])
        rows[base + (seen[base],)] = row
        seen[base] += 1
    return rows


before, after = load(sys.argv[1]), load(sys.argv[2])
moves = collections.Counter()
for key in sorted(set(before) & set(after)):
    b, a = before[key].get("combined"), after[key].get("combined")
    if b != a:
        moves[f"{b} -> {a}"] += 1
        print(f"{key[1]}#{key[2]} ({key[0]}): {b} -> {a}")
print(json.dumps(dict(moves)))
