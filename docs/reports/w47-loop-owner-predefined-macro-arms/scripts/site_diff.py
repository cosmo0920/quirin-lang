#!/usr/bin/env python3
"""Every census site whose verdict or first reason differs between two readings.

Usage: site_diff.py BEFORE.json[.gz] AFTER.json[.gz]

Prints the counts, edges, seeds and unstructured files of both, then each
moved site (file, line, callee: verdict and first reason before and after,
the after contexts when discharged). Read-only.
"""

import collections
import gzip
import json
import sys


def load(path):
    data = open(path, "rb").read()
    return json.loads(gzip.decompress(data) if path.endswith(".gz") else data)


def summary(data):
    counts = collections.Counter(s["result"]["verdict"] for s in data["sites"])
    return (f"sites {dict(sorted(counts.items()))}; edges {data['edges']}; "
            f"seeds {data['unresolved_seeds']}; functions {data['functions']}; "
            f"unstructured {data['unstructured_files']}")


def main():
    before, after = load(sys.argv[1]), load(sys.argv[2])
    print("before:", summary(before))
    print("after: ", summary(after))
    old = {(s["file"], s["line"], s["callee"]): s["result"] for s in before["sites"]}
    new = {(s["file"], s["line"], s["callee"]): s["result"] for s in after["sites"]}
    if old.keys() != new.keys():
        print("site sets differ:", sorted(old.keys() ^ new.keys()))
    moved = 0
    for key in sorted(old.keys() & new.keys()):
        a, b = old[key], new[key]
        first = lambda r: (r.get("reasons") or [None])[0]
        if a["verdict"] != b["verdict"] or first(a) != first(b):
            moved += 1
            print(f"MOVED {key[0]}:{key[1]} {key[2]}: {a['verdict']} -> {b['verdict']}")
            print(f"   before: {str(first(a) or a.get('context') or a.get('contexts'))[:400]}")
            print(f"   after:  {str(first(b) or b.get('context') or b.get('contexts'))[:400]}")
    print(f"sites moved (verdict or first reason): {moved}")


if __name__ == "__main__":
    main()
