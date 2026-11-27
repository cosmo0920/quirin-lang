#!/usr/bin/env python3
"""Where a run's source-effects artifact differs from a baseline's.

Usage: effects_paths.py RUN_DIR BASELINE_ARTIFACT_DIR

Walks both JSON documents in parallel and prints every differing leaf path
with both values (truncated), so a difference can be shown to be confined to
checkout-path-derived fields.
"""

import gzip
import json
import pathlib
import sys


def walk(a, b, path, out):
    if type(a) is not type(b):
        out.append((path, a, b))
    elif isinstance(a, dict):
        for key in sorted(set(a) | set(b)):
            walk(a.get(key), b.get(key), f"{path}/{key}", out)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append((f"{path}#len", len(a), len(b)))
        for index, (x, y) in enumerate(zip(a, b)):
            walk(x, y, f"{path}[{index}]", out)
    elif a != b:
        out.append((path, a, b))


def main() -> int:
    run = json.loads((pathlib.Path(sys.argv[1]) / "resource-diagnostics.source-effects.json").read_text())
    base = json.loads(gzip.decompress((pathlib.Path(sys.argv[2]) / "source-effects.json.gz").read_bytes()))
    out = []
    walk(run, base, "", out)
    for path, a, b in out:
        print(path, "|", str(a)[:110], "|", str(b)[:110])
    print(f"{len(out)} differing leaves")
    return 0


if __name__ == "__main__":
    sys.exit(main())
