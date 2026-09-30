#!/usr/bin/env python3
"""CG08-CASTS-AND-TYPE-QUERIES: the CG00 frozen candidates (predictions/CG08.md) on a run.

usage: candidates.py RUN   (a run directory or diagnostics file)
Prints each candidate's status and its unsupported leaves (description, line).
"""
import collections
import json
import pathlib
import re
import sys

here = pathlib.Path(__file__).resolve().parent
repo = here.parents[3]
prediction = (repo / "docs/reports/cg00-clang-gap-baseline/predictions/CG08.md").read_text()
names = re.findall(r"^- `([^`]+)::([^`]+)` —", prediction, re.M)
run = pathlib.Path(sys.argv[1])
if run.is_dir():
    run = run / "resource-diagnostics.json"
data = json.loads(run.read_text())
index = {}
for unit in data["units"]:
    for function in unit["functions"]:
        index[(unit["file"], function["function"])] = function
counts = collections.Counter()
for unit_suffix, name in names:
    hits = [f for (u, n), f in index.items() if n == name and (u == unit_suffix or u.endswith("/" + unit_suffix))]
    if len(hits) != 1:
        print(f"{unit_suffix}::{name}: {len(hits)} matches")
        continue
    f = hits[0]
    counts[f["status"]] += 1
    leaves = collections.Counter(
        (d.get("category"), d.get("description")) for d in f.get("diagnostics", []))
    print(f"{unit_suffix}::{name}: {f['status']}")
    for (category, description), n in sorted(leaves.items(), key=str):
        print(f"    {n} x {category}: {description}")
print(dict(counts), len(names), "candidates")
