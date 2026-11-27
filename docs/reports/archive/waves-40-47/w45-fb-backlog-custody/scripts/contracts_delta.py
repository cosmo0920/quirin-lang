#!/usr/bin/env python3
"""Structural delta of two arbiter contract sidecars after a root alias.

Usage: contracts_delta.py AFTER_CONTRACTS BEFORE_CONTRACTS ROOT_FROM ROOT_TO

Every occurrence of ROOT_FROM in AFTER is rewritten to ROOT_TO before the two
JSON documents are compared, so the worktree path is not a difference. Prints
each path at which the documents differ (keys only in one side, and changed
leaves), truncated.
"""

import gzip
import json
import sys


def load(path, alias=None):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        text = handle.read()
    if alias:
        text = text.replace(alias[0], alias[1])
    return json.loads(text)


def walk(after, before, path, out):
    if type(after) is not type(before):
        out.append(f"type differs at {path}")
        return
    if isinstance(after, dict):
        for key in sorted(set(after) | set(before)):
            if key not in before:
                out.append(f"only after: {path}/{key}")
            elif key not in after:
                out.append(f"only before: {path}/{key}")
            else:
                walk(after[key], before[key], f"{path}/{key}", out)
    elif isinstance(after, list):
        if len(after) != len(before):
            out.append(f"length {len(before)} -> {len(after)} at {path}")
        for index, (left, right) in enumerate(zip(after, before)):
            walk(left, right, f"{path}[{index}]", out)
    elif after != before:
        out.append(f"changed at {path}: {str(before)[:80]!r} -> {str(after)[:80]!r}")


def main():
    after = load(sys.argv[1], (sys.argv[3], sys.argv[4]))
    before = load(sys.argv[2])
    out = []
    walk(after, before, "", out)
    print(json.dumps({"differences": len(out), "paths": out[:200]}, indent=1))


if __name__ == "__main__":
    main()
