#!/usr/bin/env python3
"""List every differing JSON leaf (or JSONL line leaf) between two files,
with a count by key name, to attribute a sidecar difference.

Usage: leafdiff.py A B [--limit N]
"""
import collections
import json
import sys


def load(path):
    text = open(path).read()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return [json.loads(line) for line in text.splitlines() if line.strip()]


def walk(a, b, path, out):
    if isinstance(a, dict) and isinstance(b, dict):
        for key in sorted(set(a) | set(b)):
            walk(a.get(key), b.get(key), path + [str(key)], out)
    elif isinstance(a, list) and isinstance(b, list) and len(a) == len(b):
        for index, (x, y) in enumerate(zip(a, b)):
            walk(x, y, path + [str(index)], out)
    elif a != b:
        out.append((path, a, b))


diffs = []
walk(load(sys.argv[1]), load(sys.argv[2]), [], diffs)
limit = int(sys.argv[4]) if len(sys.argv) > 4 else 12
by_key = collections.Counter(p[-1] if p else "<root>" for p, _, _ in diffs)
print("differing leaves:", len(diffs))
print("by key:", dict(by_key))
for path, a, b in diffs[:limit]:
    print("/".join(path), "|", str(a)[:160], "|", str(b)[:160])
