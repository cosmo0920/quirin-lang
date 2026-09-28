#!/usr/bin/env python3
"""parity_multiset.py A B [--alias FROM=TO ...] -- compare two typed-path
parity streams (`parity.jsonl`, raw or `.gz`) as multisets of records (the
stream is written in completion order under `--jobs N`, so record order is
not part of its content). Exit 0 when the multisets are equal."""
import gzip
import hashlib
import pathlib
import sys
from collections import Counter


def lines(path, aliases):
    path = pathlib.Path(path)
    data = path.read_bytes() if path.is_file() else gzip.decompress(
        pathlib.Path(str(path) + ".gz").read_bytes())
    text = data.decode("utf-8")
    for source, target in aliases:
        text = text.replace(source, target)
    return Counter(text.splitlines())


args = sys.argv[1:]
aliases = []
while "--alias" in args:
    index = args.index("--alias")
    source, target = args[index + 1].split("=", 1)
    aliases.append((source, target))
    del args[index : index + 2]
a, b = lines(args[0], aliases), lines(args[1], aliases)
digest = lambda c: hashlib.sha256("\n".join(sorted(c.elements())).encode()).hexdigest()
print("records", sum(a.values()), sum(b.values()))
print("sorted sha256", digest(a), digest(b))
only_a, only_b = a - b, b - a
print("only in A", sum(only_a.values()), "only in B", sum(only_b.values()))
for line in list(only_a.elements())[:3]:
    print("  A", line[:300])
for line in list(only_b.elements())[:3]:
    print("  B", line[:300])
sys.exit(0 if a == b else 1)
