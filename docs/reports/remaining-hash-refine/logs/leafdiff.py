#!/usr/bin/env python3
"""Exhaustive leaf walk of one artifact against its archived baseline."""
import gzip
import json
import pathlib
import sys

archived = json.loads(gzip.open(pathlib.Path(sys.argv[1])).read())
measured = json.loads(pathlib.Path(sys.argv[2]).read_bytes())


def leaves(node, path=()):
    if isinstance(node, dict):
        for k in node:
            yield from leaves(node[k], path + (str(k),))
    elif isinstance(node, list):
        for i, v in enumerate(node):
            yield from leaves(v, path + (f"[{i}]",))
    else:
        yield path, node


A = dict(leaves(archived))
M = dict(leaves(measured))
added = sorted(M.keys() - A.keys())
removed = sorted(A.keys() - M.keys())
changed = sorted(k for k in A.keys() & M.keys() if A[k] != M[k])
print(f"leaves archived={len(A)} measured={len(M)}")
print(f"added={len(added)} removed={len(removed)} changed={len(changed)}")
for k in added:
    print("  + ", "/".join(k), "=", repr(M[k])[:100])
for k in removed:
    print("  - ", "/".join(k), "=", repr(A[k])[:100])
names = {}
for k in changed:
    names.setdefault(k[-1], []).append(k)
print("\nchanged leaves by final key name:")
for name in sorted(names):
    print(f"  {name}: {len(names[name])}")
print("\nfirst 40 changed leaves:")
for k in changed[:40]:
    print("  ", "/".join(k))
    print("      archived", repr(A[k])[:90])
    print("      measured", repr(M[k])[:90])
