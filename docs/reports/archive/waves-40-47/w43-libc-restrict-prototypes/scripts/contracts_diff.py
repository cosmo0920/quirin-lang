#!/usr/bin/env python3
"""Itemise how two runs' `resource-diagnostics.contracts.json` differ.

Usage: contracts_diff.py BEFORE_DIR AFTER_DIR

Walks both documents and prints every JSON path whose value differs, is added
or is removed, with long string values abbreviated to their length.
"""
import json
import pathlib
import sys


def walk(a, b, path, out):
    if isinstance(a, dict) and isinstance(b, dict):
        for key in sorted(set(a) | set(b)):
            if key not in a:
                out.append(("added", f"{path}/{key}"))
            elif key not in b:
                out.append(("removed", f"{path}/{key}"))
            else:
                walk(a[key], b[key], f"{path}/{key}", out)
    elif isinstance(a, list) and isinstance(b, list) and len(a) == len(b):
        for index, (x, y) in enumerate(zip(a, b)):
            walk(x, y, f"{path}[{index}]", out)
    elif a != b:
        describe = lambda v: f"<{len(v)} chars>" if isinstance(v, str) and len(v) > 80 else (
            f"<list of {len(v)}>" if isinstance(v, list) else v)
        out.append(("changed", f"{path}: {describe(a)} -> {describe(b)}"))


def main() -> int:
    name = "resource-diagnostics.contracts.json"
    a = json.loads((pathlib.Path(sys.argv[1]) / name).read_text())
    b = json.loads((pathlib.Path(sys.argv[2]) / name).read_text())
    out = []
    walk(a, b, "", out)
    for kind, text in out:
        print(kind, text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
