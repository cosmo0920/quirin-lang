#!/usr/bin/env python3
"""jsondiff.py A B [LIMIT] -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES: every path at
which two JSON documents (a run directory's resource-diagnostics.json, or any
file) differ, up to LIMIT (default 40), with a count of differing paths by
their shape (indices erased). Used to attribute a report digest that moved
with no row moving."""
import collections
import json
import pathlib
import re
import sys


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    return json.loads(path.read_text())


def walk(a, b, path, out):
    if type(a) is not type(b):
        out.append((path, a, b))
    elif isinstance(a, dict):
        for key in sorted(set(a) | set(b)):
            walk(a.get(key, "<absent>"), b.get(key, "<absent>"), f"{path}.{key}", out)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append((path + "[len]", len(a), len(b)))
        for index, (x, y) in enumerate(zip(a, b)):
            walk(x, y, f"{path}[{index}]", out)
    elif a != b:
        out.append((path, a, b))


def main():
    out = []
    walk(load(sys.argv[1]), load(sys.argv[2]), "", out)
    limit = int(sys.argv[3]) if len(sys.argv) > 3 else 40
    shapes = collections.Counter(re.sub(r"\[\d+\]", "[]", path) for path, _, _ in out)
    print(len(out), "differing paths")
    for shape, count in shapes.most_common(20):
        print(f"  {count:6d} {shape}")
    for path, a, b in out[:limit]:
        print(path, "|", str(a)[:200], "|", str(b)[:200])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
