#!/usr/bin/env python3
"""Classify the differing lines of a run's file against an archived one.

Usage: line_diff.py RUN_FILE ARCHIVE_FILE[.gz] [--sorted]

Prints each differing line pair's JSON key (the text before the first `:`),
tallied, and exits 0. `--sorted` compares the sorted line multisets (a
parity log written in completion order). Read-only.
"""

import collections
import gzip
import pathlib
import sys


def lines(path):
    data = pathlib.Path(path).read_bytes()
    if path.endswith(".gz"):
        data = gzip.decompress(data)
    return data.decode("utf-8", "replace").splitlines()


def main():
    ours, theirs = lines(sys.argv[1]), lines(sys.argv[2])
    if "--sorted" in sys.argv[3:]:
        print("sorted multisets equal:", sorted(ours) == sorted(theirs))
        return
    keys = collections.Counter()
    for a, b in zip(ours, theirs):
        if a != b:
            keys[a.strip().split(":", 1)[0]] += 1
    print(f"lines {len(ours)} vs {len(theirs)}; differing keys: {dict(keys)}")


if __name__ == "__main__":
    main()
