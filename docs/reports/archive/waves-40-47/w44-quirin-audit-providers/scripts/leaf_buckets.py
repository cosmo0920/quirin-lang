#!/usr/bin/env python3
"""Bucket the differing leaves of two JSON artifacts by their final key.

Usage: leaf_buckets.py LEFT.json RIGHT.json

Array indices are dropped from the bucket name, so `units[3]/x/y` counts under
`y`. Prints the total, the per-bucket counts and one example path per bucket.
Exit 0 always; the caller reads the buckets.
"""
import json
import re
import sys
from collections import Counter


def leaves(value, path=''):
    if isinstance(value, dict):
        for key in sorted(value):
            yield from leaves(value[key], f'{path}/{key}')
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from leaves(item, f'{path}[{index}]')
    else:
        yield path, value


def main():
    left = dict(leaves(json.load(open(sys.argv[1]))))
    right = dict(leaves(json.load(open(sys.argv[2]))))
    differing = sorted(p for p in set(left) | set(right) if left.get(p, '<absent>') != right.get(p, '<absent>'))
    buckets = Counter()
    example = {}
    for path in differing:
        name = re.sub(r'\[\d+\]', '', path.rsplit('/', 1)[-1]) or path
        buckets[name] += 1
        example.setdefault(name, path)
    print(json.dumps({'differing_leaves': len(differing),
                      'buckets': {k: {'count': v, 'example': example[k]} for k, v in buckets.most_common()}},
                     indent=2))


if __name__ == '__main__':
    main()
