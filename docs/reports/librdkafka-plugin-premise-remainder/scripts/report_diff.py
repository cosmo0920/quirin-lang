#!/usr/bin/env python3
"""report_diff.py A.json B.json -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (copied from LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED): every
JSON path at which two route reports differ (leaf values, list lengths), so
the report can name what moved besides the rows. Prints at most 40 paths."""
import json
import sys


def walk(a, b, path, out):
    if type(a) is not type(b):
        out.append((path, repr(a)[:80], repr(b)[:80]))
    elif isinstance(a, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                out.append((f"{path}.{k}", "absent" if k not in a else "present", "absent" if k not in b else "present"))
            else:
                walk(a[k], b[k], f"{path}.{k}", out)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append((path, f"len {len(a)}", f"len {len(b)}"))
        for i, (x, y) in enumerate(zip(a, b)):
            walk(x, y, f"{path}[{i}]", out)
    elif a != b:
        out.append((path, repr(a)[:80], repr(b)[:80]))


out = []
walk(json.load(open(sys.argv[1])), json.load(open(sys.argv[2])), "$", out)
import collections
import re

print(len(out), "differences")
kinds = collections.Counter(re.sub(r"\[\d+\]", "[]", re.sub(r"clauses\.[^.]+\.", "clauses.F.", p)) for p, _, _ in out)
for kind, count in kinds.most_common():
    print(count, kind)
if "-v" in sys.argv:
    for row in out[:40]:
        print(*row)
