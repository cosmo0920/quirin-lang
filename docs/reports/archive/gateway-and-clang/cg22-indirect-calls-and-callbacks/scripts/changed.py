#!/usr/bin/env python3
"""changed.py BEFORE AFTER -- every function whose status or leaves differ
between two runs, with its leaf delta (compare.py's reading)."""
import collections
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import compare  # noqa: E402

_, before = compare.load(pathlib.Path(sys.argv[1]))
_, after = compare.load(pathlib.Path(sys.argv[2]))
for key in sorted(set(before) | set(after)):
    if before.get(key) == after.get(key):
        continue
    b_status, b_leaves = before.get(key, ("absent", collections.Counter()))
    a_status, a_leaves = after.get(key, ("absent", collections.Counter()))
    delta = collections.Counter(a_leaves)
    delta.subtract(b_leaves)
    print(key[0], key[1], b_status, "->", a_status,
          {f"{k[0]}: {k[1]}": v for k, v in delta.items() if v})
