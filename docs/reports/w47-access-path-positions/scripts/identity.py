#!/usr/bin/env python3
"""Byte identity of two run directories' artifacts, raw and after a root alias.

Usage: identity.py BASELINE_DIR RUN_DIR OUT_JSON [--alias FROM=TO ...]

Every file present in BASELINE_DIR (plain or `.gz`) is compared with the
file of the same name in RUN_DIR. `parity.jsonl` is compared as a sorted
multiset of rows, since worker arrival order is not part of the record.
"""

import gzip
import hashlib
import json
import pathlib
import sys


def read(path):
    if path.exists():
        return path.read_bytes()
    return gzip.decompress(path.with_name(path.name + ".gz").read_bytes())


def main():
    baseline, run, out = map(pathlib.Path, sys.argv[1:4])
    aliases = []
    arguments = sys.argv[4:]
    while arguments:
        if arguments[0] == "--alias":
            source, target = arguments[1].split("=", 1)
            aliases.append((source.encode(), target.encode()))
            arguments = arguments[2:]
        else:
            raise SystemExit(f"unknown argument {arguments[0]}")
    names = sorted({p.name[:-3] if p.name.endswith(".gz") else p.name for p in baseline.iterdir()})
    result = {}
    for name in names:
        if name in ("run.json", "audit.log", "console.log", "stderr.log", "exit", "manifest.json"):
            continue
        try:
            old, new = read(baseline / name), read(run / name)
        except FileNotFoundError:
            result[name] = {"present_in_run": False}
            continue
        if name.startswith("parity.jsonl"):
            old = b"".join(sorted(old.splitlines(keepends=True)))
            new = b"".join(sorted(new.splitlines(keepends=True)))
        aliased = new
        for source, target in aliases:
            aliased = aliased.replace(source, target)
        result[name] = {
            "baseline_sha256": hashlib.sha256(old).hexdigest(),
            "run_sha256": hashlib.sha256(new).hexdigest(),
            "byte_identical": old == new,
            "identical_after_root_alias": old == aliased,
            "sorted_rows": name.startswith("parity.jsonl"),
        }
    pathlib.Path(out).write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    print(json.dumps(result, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
