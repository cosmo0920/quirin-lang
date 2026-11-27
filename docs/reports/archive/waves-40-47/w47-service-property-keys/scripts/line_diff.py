#!/usr/bin/env python3
"""Classify the differing lines of a run's file against an archived one.

Usage: line_diff.py RUN_FILE ARCHIVE_FILE[.gz]

Prints, per JSON key the differing lines carry (the text before the first
colon, stripped), how many lines differ, and one example pair. Read-only.
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
    print(f"line counts {len(ours)} / {len(theirs)}")
    kinds = collections.Counter()
    example = {}
    for a, b in zip(ours, theirs):
        if a != b:
            key = a.strip().split(":", 1)[0]
            kinds[key] += 1
            example.setdefault(key, (a.strip()[:200], b.strip()[:200]))
    for key, count in kinds.most_common():
        print(f"{count:6d}  {key}")
        print(f"        run:     {example[key][0]}")
        print(f"        archive: {example[key][1]}")


if __name__ == "__main__":
    main()
