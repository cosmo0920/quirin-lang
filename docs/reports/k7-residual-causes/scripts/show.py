#!/usr/bin/env python3
"""Print recensus.json's freed rows with their K7 leaves (reading aid)."""
import json
import sys

d = json.load(open(sys.argv[1]))
for key in ("freed_rows", "loose_rows"):
    print("==", key)
    for r in sorted(d[key], key=lambda r: (r["items"], r["file"])):
        print(r["file"], r["function"], r["items"])
        for leaf in r["k7"]:
            facts = leaf["facts"] or {}
            print("   ", leaf["origin"], leaf["family"], leaf["tags"], facts.get("roots"), facts.get("values"),
                  facts.get("accesses"), facts.get("positions"), facts.get("constructs"), facts.get("max_hops"))
print("== frozen changed")
for f in d["frozen_11_07"]:
    for r in f["rows"]:
        if not r["freed_by_k7"]:
            print(f["function"], r["status"], r["items"])
