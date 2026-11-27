#!/usr/bin/env python3
"""jsondiff.py A B [LIMIT] -- the JSON paths whose values differ between two
reports (for a digest change with no custody change), at most LIMIT lines."""
import json
import sys


def walk(a, b, path, out):
    if type(a) is not type(b):
        out.append((path, a, b))
    elif isinstance(a, dict):
        for key in sorted(set(a) | set(b)):
            walk(a.get(key), b.get(key), f'{path}.{key}', out)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append((f'{path}[len]', len(a), len(b)))
        for index, (x, y) in enumerate(zip(a, b)):
            walk(x, y, f'{path}[{index}]', out)
    elif a != b:
        out.append((path, a, b))


left = json.load(open(sys.argv[1]))
right = json.load(open(sys.argv[2]))
limit = int(sys.argv[3]) if len(sys.argv) > 3 else 40
differences = []
walk(left, right, '$', differences)
print(len(differences), 'differences')
for path, a, b in differences[:limit]:
    print(path, '|', str(a)[:150], '|', str(b)[:150])
