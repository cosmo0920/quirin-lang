#!/usr/bin/env python3
"""Compare two runs' artifacts after rewriting one checkout root to another.

Usage: path_alias_diff.py RUN_A RUN_B ROOT_A ROOT_B

RUN_A was produced from a checkout at ROOT_A, RUN_B from ROOT_B. Every
occurrence of ROOT_A in RUN_A's artifacts is rewritten to ROOT_B; for the JSON
artifacts, every value that is a 64-hex digest is additionally reported by the
key path where the two runs still differ, so a path-derived digest difference
(`build_digest`, `resource_contracts:<digest>`, ...) is listed rather than
hidden. Prints, per artifact: identical raw, identical after the rewrite, or
the differing key paths.
"""

import json
import pathlib
import re
import sys

NAMES = [
    "resource-diagnostics.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.source-effects.json",
    "monkey-source.json",
    "resource-diagnostics.contracts.json",
]
HEX = re.compile(r"[0-9a-f]{64}")


def diff(a, b, path, out):
    if type(a) is not type(b):
        out.append(path)
    elif isinstance(a, dict):
        for key in sorted(set(a) | set(b)):
            if key not in a or key not in b:
                out.append(f"{path}/{key} (present on one side)")
            else:
                diff(a[key], b[key], f"{path}/{key}", out)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append(f"{path} (length)")
        for index, (x, y) in enumerate(zip(a, b)):
            diff(x, y, f"{path}[{index}]", out)
    elif a != b:
        out.append(path)


def main() -> int:
    run_a, run_b, root_a, root_b = sys.argv[1:5]
    for name in NAMES:
        raw_a = (pathlib.Path(run_a) / name).read_bytes()
        raw_b = (pathlib.Path(run_b) / name).read_bytes()
        if raw_a == raw_b:
            print(f"{name}: identical")
            continue
        rewritten = raw_a.replace(root_a.encode(), root_b.encode())
        if rewritten == raw_b:
            print(f"{name}: identical after the root rewrite")
            continue
        out: list = []
        diff(json.loads(rewritten), json.loads(raw_b), "", out)
        generic = sorted({re.sub(r"\[\d+\]", "[]", HEX.sub("<digest>", p)) for p in out})
        print(f"{name}: {len(out)} differing paths after the root rewrite")
        for path in generic[:40]:
            print(f"   {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
