#!/usr/bin/env python3
"""Group the differing JSON leaves of two runs by their final key (r1 pair)."""
import collections
import json
import pathlib
import re
import sys

left, right = (pathlib.Path(a).resolve() for a in sys.argv[1:3])


def leaves(value, path="$"):
    if isinstance(value, dict):
        for key, item in value.items():
            yield from leaves(item, f"{path}.{key}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from leaves(item, f"{path}[{index}]")
    else:
        yield path, value


for name in sys.argv[3:]:
    docs = []
    for side in (left, right):
        text = (side / name).read_text()
        if name.endswith(".jsonl"):
            docs.append({f"line{i}": json.loads(l) for i, l in enumerate(text.splitlines())})
        else:
            docs.append(json.loads(text))
    la, lb = dict(leaves(docs[0])), dict(leaves(docs[1]))
    kinds = collections.Counter()
    for k in set(la) | set(lb):
        if la.get(k) != lb.get(k):
            kinds[re.sub(r"\[\d+\]", "[]", re.sub(r"line\d+", "line", k)).split(".")[-1]] += 1
    print(name, dict(kinds.most_common(12)))
