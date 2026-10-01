#!/usr/bin/env python3
"""CG30-DIALECTS-FLAGS-TYPES: every JSON field that differs between two
resource-diagnostics.json reports, grouped by its path with array indices
removed, after removing the BEFORE tree's path (`private/cg30dft/base-tree/`).

usage: fields.py BEFORE.json AFTER.json
"""

import collections
import json
import pathlib
import re
import sys


def load(path):
    text = pathlib.Path(path).read_text().replace("private/cg30dft/base-tree/", "")
    return json.loads(text)


def walk(left, right, path, out):
    if type(left) is not type(right):
        out[path] += 1
    elif isinstance(left, dict):
        for key in set(left) | set(right):
            if key not in left or key not in right:
                out[f"{path}/{key} (missing)"] += 1
            else:
                walk(left[key], right[key], f"{path}/{key}", out)
    elif isinstance(left, list):
        if len(left) != len(right):
            out[f"{path} (length)"] += 1
        else:
            for a, b in zip(left, right):
                walk(a, b, path, out)
    elif left != right:
        out[path] += 1


def main():
    out = collections.Counter()
    walk(load(sys.argv[1]), load(sys.argv[2]), "", out)
    for path, count in sorted(out.items()):
        print(count, re.sub(r"\[\d+\]", "", path))


if __name__ == "__main__":
    main()
