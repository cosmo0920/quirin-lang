#!/usr/bin/env python3
"""arm_sites_summary.py SITES.json -- summarise arm_sites.py's output: per
site kind and arm, how many sites carry local type evidence (an enclosing
`switch` on `rko_type`, a constructor with a constant type in the same
function, or an assertion earlier in it) and how many carry none."""
import collections
import json
import sys

sites = json.load(open(sys.argv[1]))
errors = [s for s in sites if "error" in s]
for s in errors:
    print("ERROR", s["unit"], s["error"][:200])
rows = collections.Counter()
for s in sites:
    if "error" in s:
        continue
    e = s["evidence"]
    if e["switch_cases"]:
        evidence = "switch"
    elif e["constructed"] and "None" not in e["constructed"]:
        evidence = "constructed(const)"
    elif e["constructed"]:
        evidence = "constructed(var)"
    elif e["asserted"]:
        evidence = "assert"
    else:
        evidence = "none"
    rows[(s["what"], s["arm"].split(".")[0], evidence)] += 1
for (what, arm, evidence), count in sorted(rows.items()):
    print(f"{count:4d}  {what:24s} {arm:22s} {evidence}")
print("total", sum(rows.values()))
if len(sys.argv) > 2:
    for s in sites:
        if "error" not in s and s["arm"].split(".")[0] in sys.argv[2:]:
            print(s["file"], s["line"], s["function"], s["what"], s["arm"], s["base"], s["evidence"])
