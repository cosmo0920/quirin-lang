#!/usr/bin/env python3
"""Classify the differing JSON leaves of a run's sidecar against the archive's.

Usage: sidecar_leaves.py RUN_FILE ARCHIVE_GZ RUN_ROOT BASE_ROOT

A leaf is `path` when it is equal after replacing the run's worktree root
with the baseline's; `digest` when both values are hexadecimal digests or
`resource_contracts:` ids (bound to those paths); anything else is `other`
and printed. The round-1 cards attributed such differences to the cwd.
"""
import collections
import gzip
import json
import re
import sys

DIGEST = re.compile(r'^(resource_contracts:)?[0-9a-f]{16,64}$')


def leaves(value, path=''):
    if isinstance(value, dict):
        for key, item in value.items():
            yield from leaves(item, f'{path}/{key}')
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from leaves(item, f'{path}[{index}]')
    else:
        yield path, value


def normal(value, run_root, base_root):
    return value.replace(run_root, base_root) if isinstance(value, str) else value


run_file, archive, run_root, base_root = sys.argv[1:5]
run_text = open(run_file).read().replace(run_root, base_root)
base_text = gzip.open(archive).read().decode()
run = dict(leaves(json.loads(run_text)))
base = dict(leaves(json.loads(base_text)))
classes = collections.Counter()
others = []
for key in sorted(run.keys() | base.keys()):
    left, right = base.get(key), run.get(key)
    if left == right:
        continue
    if isinstance(left, str) and isinstance(right, str) and DIGEST.match(left) and DIGEST.match(right):
        classes['digest'] += 1
    else:
        classes['other'] += 1
        others.append((key, left, right))
print(json.dumps({'after_root_alias': dict(classes)}))
for key, left, right in others[:40]:
    print(key, str(left)[:70], '->', str(right)[:70])
