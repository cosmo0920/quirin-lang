#!/usr/bin/env python3
"""route_diff.py BEFORE.json AFTER.json -- where two arbiter reports differ:
every JSON path (dict keys, list indices) whose value differs, collapsed by
replacing list indices with `[]`, with its count. Read-only; prints JSON.
"""
import json
import sys
from collections import Counter


def walk(a, b, path, out):
    if type(a) is not type(b):
        out[path] += 1
        return
    if isinstance(a, dict):
        for key in set(a) | set(b):
            if key not in a or key not in b:
                out[f"{path}.{key}"] += 1
            else:
                walk(a[key], b[key], f"{path}.{key}", out)
    elif isinstance(a, list):
        if len(a) != len(b):
            out[f"{path}[len]"] += 1
        for x, y in zip(a, b):
            walk(x, y, f"{path}[]", out)
    elif a != b:
        out[path] += 1


def main():
    a = json.load(open(sys.argv[1]))
    b = json.load(open(sys.argv[2]))
    out = Counter()
    walk(a, b, "$", out)
    json.dump(dict(sorted(out.items())), sys.stdout, indent=1)
    print()


if __name__ == "__main__":
    main()
