#!/usr/bin/env python3
"""json_diff.py A B  (each .json or .json.gz): print every differing leaf path.

Used to attribute a non-identical sidecar: each printed path is a leaf whose
value differs, with both values truncated. Exit 0 always; the reader decides.
"""
import gzip
import json
import pathlib
import sys


def load(path):
    path = pathlib.Path(path)
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def walk(a, b, path, out):
    if type(a) is not type(b):
        out.append((path, a, b))
    elif isinstance(a, dict):
        for key in sorted(set(a) | set(b)):
            walk(a.get(key, "<absent>"), b.get(key, "<absent>"), f"{path}.{key}", out)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append((path + "#len", len(a), len(b)))
        for index, (x, y) in enumerate(zip(a, b)):
            walk(x, y, f"{path}[{index}]", out)
    elif a != b:
        out.append((path, a, b))


def main():
    out = []
    walk(load(sys.argv[1]), load(sys.argv[2]), "$", out)
    print(f"{len(out)} differing leaves")
    for path, a, b in out[:200]:
        print(path, "|", str(a)[:120], "|", str(b)[:120])


if __name__ == "__main__":
    main()
