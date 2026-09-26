#!/usr/bin/env python3
"""Compare two preparation output trees file by file.

Usage: compare_trees.py LEFT RIGHT OUT_JSON [--normalize] [--host-files NAME...]

Every regular file of both trees is listed (symbolic links by target). A file
present in both is `identical` when its bytes are; otherwise, with
`--normalize`, it is compared again after replacing LEFT's root spelling with
RIGHT's (`identical-after-normalization`), else it `differs`. Files only in
RIGHT named by `--host-files` are the host's own records (the report, its
invocation record and the captured standard error), listed apart. Writes
OUT_JSON with the counts, every differing or one-sided file, and the SHA-256 of
the key files; exits 0 whatever it finds.
"""

import hashlib
import json
import os
import pathlib
import sys

KEY_FILES = [
    "libevent-clang-build/compile_commands.json",
    "libevent-clang-build/libevent-core-audit.json.config",
    "libevent-clang-build/libevent-build-provenance.json",
    "libevent-clang-build/CMakeCache.txt",
    "libevent-clang-build/lib/libevent_core.a",
    "libevent-clang-build/configure.log",
    "libevent-clang-build/native-build.log",
]


def files(root):
    listed = {}
    for directory, _, names in os.walk(root):
        for name in names:
            path = pathlib.Path(directory) / name
            relative = path.relative_to(root).as_posix()
            if path.is_symlink():
                listed[relative] = ("link", os.readlink(path).encode())
            elif path.is_file():
                listed[relative] = ("file", path.read_bytes())
    return listed


def main():
    left = pathlib.Path(sys.argv[1]).resolve()
    right = pathlib.Path(sys.argv[2]).resolve()
    out = pathlib.Path(sys.argv[3])
    rest = sys.argv[4:]
    normalize = "--normalize" in rest
    host = set(rest[rest.index("--host-files") + 1:]) if "--host-files" in rest else set()
    a, b = files(left), files(right)
    record = {
        "left": str(left),
        "right": str(right),
        "normalize": normalize,
        "counts": {"compared": 0, "identical": 0, "identical_after_normalization": 0,
                   "differ": 0, "only_left": 0, "only_right": 0, "host_records": 0},
        "differ": [],
        "only_left": sorted(set(a) - set(b)),
        "only_right": [],
        "host_records": [],
        "key_files": {},
    }
    for name in sorted(set(b) - set(a)):
        if name in host:
            record["host_records"].append(name)
        else:
            record["only_right"].append(name)
    record["counts"]["only_left"] = len(record["only_left"])
    record["counts"]["only_right"] = len(record["only_right"])
    record["counts"]["host_records"] = len(record["host_records"])
    for name in sorted(set(a) & set(b)):
        record["counts"]["compared"] += 1
        left_bytes, right_bytes = a[name][1], b[name][1]
        if left_bytes == right_bytes:
            record["counts"]["identical"] += 1
        elif normalize and left_bytes.replace(str(left).encode(), str(right).encode()) == right_bytes:
            record["counts"]["identical_after_normalization"] += 1
        else:
            record["counts"]["differ"] += 1
            record["differ"].append(name)
    for name in KEY_FILES:
        record["key_files"][name] = {
            side: (hashlib.sha256(tree[name][1]).hexdigest() if name in tree else None)
            for side, tree in (("left", a), ("right", b))
        }
    out.write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record["counts"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
