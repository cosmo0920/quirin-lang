#!/usr/bin/env python3
"""ARM-OBLIGATIONS-UNIT-GRAIN: which leaf keys differ between two JSON
reports of the same shape (after respell_sides.py), counted by key.

Prints `key count` lines and exits 1 when the shapes differ (a dict's keys or
a list's length), 0 otherwise. Used for the system and slice routes, where
the strict token substitution leaves list groups ambiguous: it shows that the
differing leaves are only source byte offsets and the digests computed over
the preprocessed text, which spells each side's directory.

Usage: leaf_census.py BEFORE AFTER
"""
import json
import sys
from collections import Counter


def walk(before, after, key, counts):
    if isinstance(before, dict) and isinstance(after, dict):
        if set(before) != set(after):
            return False
        return all(walk(before[k], after[k], k, counts) for k in before)
    if isinstance(before, list) and isinstance(after, list):
        if len(before) != len(after):
            return False
        return all(walk(b, a, key, counts) for b, a in zip(before, after))
    if before != after:
        counts[key] += 1
    return True


def main():
    before = json.load(open(sys.argv[1]))
    after = json.load(open(sys.argv[2]))
    counts = Counter()
    same_shape = walk(before, after, None, counts)
    for key, count in sorted(counts.items(), key=str):
        print(key, count)
    print("same shape:", same_shape)
    return 0 if same_shape else 1


if __name__ == "__main__":
    sys.exit(main())
