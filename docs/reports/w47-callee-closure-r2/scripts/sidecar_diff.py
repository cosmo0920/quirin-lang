#!/usr/bin/env python3
"""Structural difference of two contracts / source-effects sidecars.

Usage: sidecar_diff.py OLD(.json|.json.gz) NEW(.json|.json.gz) OUT_JSON [--alias FROM=TO ...]

Walks both JSON documents and records every differing path (added, removed,
changed leaf), after rewriting each alias in NEW's raw text. Long string leaves
are summarised by length and SHA-256. The output is the evidence behind a
BEFORE sidecar that is not byte-identical to its baseline: each differing path
is then attributed to a named commit by the report.
"""

import gzip
import hashlib
import json
import pathlib
import sys


def read(path):
    data = pathlib.Path(path).read_bytes()
    if str(path).endswith(".gz"):
        data = gzip.decompress(data)
    return data


def leaf(value):
    if isinstance(value, str) and len(value) > 120:
        return {"len": len(value), "sha256": hashlib.sha256(value.encode()).hexdigest()}
    return value


def walk(old, new, path, out):
    if isinstance(old, dict) and isinstance(new, dict):
        for key in sorted(set(old) | set(new)):
            child = f"{path}/{key}"
            if key not in new:
                out.append({"path": child, "kind": "removed", "old": leaf(old[key]) if not isinstance(old[key], (dict, list)) else type(old[key]).__name__})
            elif key not in old:
                out.append({"path": child, "kind": "added", "new": leaf(new[key]) if not isinstance(new[key], (dict, list)) else type(new[key]).__name__})
            else:
                walk(old[key], new[key], child, out)
        return
    if isinstance(old, list) and isinstance(new, list) and len(old) == len(new):
        for index, (a, b) in enumerate(zip(old, new)):
            walk(a, b, f"{path}[{index}]", out)
        return
    if old != new:
        out.append({"path": path, "kind": "changed", "old": leaf(old) if not isinstance(old, (dict, list)) else f"{type(old).__name__}[{len(old)}]",
                    "new": leaf(new) if not isinstance(new, (dict, list)) else f"{type(new).__name__}[{len(new)}]"})


def main():
    old_path, new_path, out_path = sys.argv[1:4]
    aliases = []
    rest = sys.argv[4:]
    while rest:
        if rest[0] != "--alias":
            raise SystemExit(f"unknown argument {rest[0]}")
        source, target = rest[1].split("=", 1)
        aliases.append((source.encode(), target.encode()))
        rest = rest[2:]
    old_raw, new_raw = read(old_path), read(new_path)
    for source, target in aliases:
        new_raw = new_raw.replace(source, target)
    old, new = json.loads(old_raw), json.loads(new_raw)
    out = []
    walk(old, new, "", out)
    document = {"old": old_path, "new": new_path, "differences": len(out), "paths": out}
    pathlib.Path(out_path).write_text(json.dumps(document, indent=1) + "\n")
    print(f"{len(out)} differing paths")
    return 0


if __name__ == "__main__":
    sys.exit(main())
