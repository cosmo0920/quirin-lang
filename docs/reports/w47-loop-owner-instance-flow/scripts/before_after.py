#!/usr/bin/env python3
"""Compare two arbiter runs' resource-diagnostics.json function by function.

Usage: before_after.py BEFORE_JSON AFTER_JSON

Prints the status tallies on both sides and, for every function whose status
or diagnostics changed, its before and after status and the diagnostics
added and removed (their message text). Read-only; exits 0.
"""

import collections
import json
import sys


def rows(path):
    data = json.load(open(path))
    out = {}
    for unit in data.get("units", []):
        for function in unit.get("functions", []):
            key = (unit.get("path") or unit.get("file") or unit.get("source", ""),
                   function.get("name") or function.get("function"))
            out.setdefault(key, []).append(function)
    return out


def text(function):
    return sorted(json.dumps(d, sort_keys=True) for d in function.get("diagnostics", []))


def main():
    before, after = rows(sys.argv[1]), rows(sys.argv[2])
    tally = lambda r: dict(collections.Counter(
        f.get("status") for fs in r.values() for f in fs))
    print("before", tally(before))
    print("after ", tally(after))
    for key in sorted(set(before) | set(after)):
        b, a = before.get(key, []), after.get(key, [])
        if [text(f) for f in b] == [text(f) for f in a] and \
                [f.get("status") for f in b] == [f.get("status") for f in a]:
            continue
        print(f"== {key[0]} {key[1]}: {[f.get('status') for f in b]} -> {[f.get('status') for f in a]}")
        bt = collections.Counter(t for f in b for t in text(f))
        at = collections.Counter(t for f in a for t in text(f))
        for t in sorted((bt - at).elements()):
            print("   - ", t[:400])
        for t in sorted((at - bt).elements()):
            print("   + ", t[:400])


if __name__ == "__main__":
    main()
