#!/usr/bin/env python3
"""provenance_diff.py ARCHIVE.json.gz RUN.json: classify every differing line of
a resource-diagnostics.json against its archived copy by JSON key. Prints the
count per key; exits 0 when every differing line is one of the provenance
digests (`resource_contract_sources`, `inventory_sha256`), 1 otherwise."""
import collections
import gzip
import re
import sys

PROVENANCE = {"resource_contract_sources", "inventory_sha256"}


def lines(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        return handle.read().splitlines()


def main():
    archived, run = lines(sys.argv[1]), lines(sys.argv[2])
    if len(archived) != len(run):
        print(f"line counts differ: {len(archived)} / {len(run)}")
        return 1
    keys = collections.Counter()
    for old, new in zip(archived, run):
        if old != new:
            match = re.match(r'\s*"([^"]+)":', new)
            keys[match[1] if match else "<unkeyed>"] += 1
    print(dict(keys))
    return 0 if set(keys) <= PROVENANCE else 1


if __name__ == "__main__":
    raise SystemExit(main())
