#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""null_tally.py BEFORE.jsonl AFTER.jsonl -- LIST-ENTRY-RELEASE-IDENTITY.

The null-property stream of a route on both sides: the tallies of the
`null`, `custody` and `combined` verdicts, and every record whose verdict
fields differ (the hypothesis context carries the commit, so it is not
compared)."""
import collections
import json
import sys


def load(path):
    out = {}
    seen = collections.Counter()
    for line in open(path):
        if not line.strip():
            continue
        r = json.loads(line)
        key = (r["file"], r["function"])
        seen[key] += 1
        null = r.get("null")
        null = null.get("verdict", null.get("status")) if isinstance(null, dict) else null
        out[key + (seen[key],)] = (str(null), r.get("custody"), r.get("combined"))
    return out


before, after = load(sys.argv[1]), load(sys.argv[2])
for name, side in (("before", before), ("after", after)):
    print(name, len(side), "null", dict(collections.Counter(v[0] for v in side.values())),
          "custody", dict(collections.Counter(v[1] for v in side.values())),
          "combined", dict(collections.Counter(v[2] for v in side.values())))
for key in sorted(set(before) | set(after)):
    if before.get(key) != after.get(key):
        print("  ", key, before.get(key), "->", after.get(key))
