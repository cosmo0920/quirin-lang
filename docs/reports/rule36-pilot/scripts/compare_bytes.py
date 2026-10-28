#!/usr/bin/env python3
"""RULE36-PILOT: byte comparison of one route's report artifacts between the
published BASE baseline and a measured side, listing each file as same or
different (with SHA-256 of both), ignoring nothing.

Usage: compare_bytes.py BASELINE_ROUTE_DIR SIDE_ROUTE_DIR
"""
import hashlib
import sys
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    base, side = Path(sys.argv[1]), Path(sys.argv[2])
    names = sorted({p.name for p in base.iterdir() if p.is_file()}
                   | {p.name for p in side.iterdir() if p.is_file()})
    differ = 0
    for name in names:
        left, right = base / name, side / name
        if not left.is_file() or not right.is_file():
            print(f"MISSING {name} base={left.is_file()} side={right.is_file()}")
            differ += 1
            continue
        same = left.read_bytes() == right.read_bytes()
        differ += not same
        print(f"{'same' if same else 'DIFF'} {name} {digest(left)[:16]} {digest(right)[:16]}")
        if not same and name.endswith(".json"):
            for key, a, b in json_differences(left, right):
                print(f"    {key}: {str(a)[:120]!s} -> {str(b)[:120]!s}")
    print(f"{len(names)} files, {differ} differ")


def flatten(value, prefix=""):
    if isinstance(value, dict):
        for key, item in value.items():
            yield from flatten(item, f"{prefix}.{key}" if prefix else key)
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from flatten(item, f"{prefix}[{index}]")
    else:
        yield prefix, value


def json_differences(left, right):
    import json
    try:
        a = dict(flatten(json.loads(left.read_text())))
        b = dict(flatten(json.loads(right.read_text())))
    except ValueError:
        return []
    return [(key, a.get(key), b.get(key)) for key in sorted(set(a) | set(b))
            if a.get(key) != b.get(key)]


if __name__ == "__main__":
    main()
