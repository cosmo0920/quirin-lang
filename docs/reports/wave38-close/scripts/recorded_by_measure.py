#!/usr/bin/env python3
"""WAVE-38-CLOSE, item 6 (MATRIX-RECORDED-BY-ABSENT-67): the cheap fact.

For each named_followups entry without `recorded_by`, does its `source` name
exactly ONE docs/reports/<dir>/README.md, and does that directory name map to
a card name that some OTHER entry already uses as `recorded_by` (upper-cased
kebab, the convention every registering card has followed)? Read-only.
"""
import json
import re
import sys
import collections

root = sys.argv[1]
m = json.load(open(root + "/docs/reviews/r0_3/acceptance_matrix.json"))
nf = m["named_followups"]
absent = [e for e in nf if "recorded_by" not in e]
known_cards = {e["recorded_by"] for e in nf if "recorded_by" in e}
print("entries without recorded_by:", len(absent))
print("distinct recorded_by values across the rest:", len(known_cards))

single, multi, none = [], [], []
for e in absent:
    dirs = sorted(set(re.findall(r"docs/reports/([A-Za-z0-9_.-]+)/README\.md", e.get("source", ""))))
    if len(dirs) == 1:
        single.append((e["id"], dirs[0]))
    elif len(dirs) > 1:
        multi.append((e["id"], dirs))
    else:
        none.append((e["id"], e.get("source")))
print("source names exactly one report directory:", len(single))
print("source names more than one report directory:", len(multi))
print("source names no report directory:", len(none))

upper_matches = [(i, d) for i, d in single if d.upper() in known_cards]
print("single-directory entries whose DIR.upper() is an existing recorded_by value:", len(upper_matches))
by_dir = collections.Counter(d for _, d in single)
print("single-directory: distinct directories", len(by_dir))
for d, c in sorted(by_dir.items()):
    print(f"  {c:2d} {d}  -> {d.upper()}  {'KNOWN' if d.upper() in known_cards else 'not a recorded_by value'}")
print("multi-directory entries:")
for i, ds in multi:
    print("  ", i, ds)
print("no-directory entries:")
for i, s in none:
    print("  ", i, s)
