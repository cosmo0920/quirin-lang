#!/usr/bin/env python3
"""Leaf paths that differ between two JSON files, with an optional path alias
(card W45-REFUTED-CONTRACT-WITHDRAWAL).

Usage: sidecar_delta.py BEFORE AFTER [--alias FROM=TO]

Prints every differing leaf path (a key present on one side only is reported
as its own path) as JSON; exit 0 always.
"""
import gzip
import json
import sys


def load(path, alias):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        text = handle.read()
    if alias:
        text = text.replace(alias[0], alias[1])
    return json.loads(text)


def walk(left, right, path, out):
    if type(left) is not type(right):
        out.append(path)
    elif isinstance(left, dict):
        for key in sorted(set(left) | set(right)):
            if key not in left or key not in right:
                out.append(f"{path}/{key}")
            elif left[key] != right[key]:
                walk(left[key], right[key], f"{path}/{key}", out)
    elif isinstance(left, list):
        if len(left) != len(right):
            out.append(f"{path}#len")
        for index, (a, b) in enumerate(zip(left, right)):
            if a != b:
                walk(a, b, f"{path}[{index}]", out)
    elif left != right:
        out.append(path)


def main():
    alias = None
    if "--alias" in sys.argv:
        alias = sys.argv[sys.argv.index("--alias") + 1].split("=", 1)
    out = []
    walk(load(sys.argv[1], alias), load(sys.argv[2], alias), "", out)
    json.dump(out, sys.stdout, indent=1)
    print()


if __name__ == "__main__":
    main()
