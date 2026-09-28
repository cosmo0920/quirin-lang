#!/usr/bin/env python3
"""Multiset difference of two typed-path parity streams (QCORE_TYPED_PATH_PARITY).

Usage: parity_diff.py BEFORE_PARITY AFTER_PARITY OUT_JSON

Rows are compared as sorted multisets (worker arrival order is not part of
the record). Reports the added and removed rows with their function, position
and outcome keys, and the SHA-256 of each sorted stream.
"""

import collections
import hashlib
import json
import pathlib
import sys


def rows(path):
    return pathlib.Path(path).read_bytes().splitlines()


def summary(row):
    data = json.loads(row)
    keys = ("unit", "file", "function", "line", "position", "decision", "outcome", "admitted", "reason")
    return {k: data[k] for k in keys if k in data} or {"keys": sorted(data)[:12]}


def main():
    before, after = rows(sys.argv[1]), rows(sys.argv[2])
    b, a = collections.Counter(before), collections.Counter(after)
    added, removed = a - b, b - a
    result = {
        "before_rows": len(before), "after_rows": len(after),
        "before_sorted_sha256": hashlib.sha256(b"\n".join(sorted(before))).hexdigest(),
        "after_sorted_sha256": hashlib.sha256(b"\n".join(sorted(after))).hexdigest(),
        "added": [dict(summary(r), count=n) for r, n in sorted(added.items())],
        "removed": [dict(summary(r), count=n) for r, n in sorted(removed.items())],
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps(result, indent=1) + "\n")
    print(f"rows {len(before)} -> {len(after)}; added {sum(added.values())}; removed {sum(removed.values())}")


if __name__ == "__main__":
    main()
