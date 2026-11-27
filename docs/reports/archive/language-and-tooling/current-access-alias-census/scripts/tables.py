#!/usr/bin/env python3
"""Card CURRENT-ACCESS-ALIAS-CENSUS: the report's tables from the census outputs.

usage: tables.py RUN_DIR ALIAS_DIR SHAPE_DIR RANK_COARSE_JSON
Prints markdown. Reads only the outputs of run_census.py, the W47 alias census,
shape_census.py and rank.py (coarse).
"""
import collections
import gzip
import json
import pathlib
import sys

run, alias_dir, shape_dir, rank_path = (pathlib.Path(a) for a in sys.argv[1:5])
report = json.load(open(run / "resource-diagnostics.json"))
rank = json.load(open(rank_path))
summary = json.load(open(alias_dir / "summary.json"))

print("## counts")
print(report["diagnostic_counts"])
status = collections.Counter(u["status"] for u in report["units"])
print("units by status", dict(status), "configurations", len(report["units"]))
fstatus = collections.Counter(f["status"] for u in report["units"] for f in u["functions"])
print("function rows by status", dict(fstatus))
unique = {}
order = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}
for u in report["units"]:
    for f in u["functions"]:
        key = (u["file"], f["function"])
        if key not in unique or order[f["status"]] > order[unique[key]]:
            unique[key] = f["status"]
print("unique functions", len(unique), dict(collections.Counter(unique.values())))

leaves = collections.defaultdict(collections.Counter)
groups = collections.Counter()
with gzip.open(shape_dir / "leaves.jsonl.gz", "rt") as handle:
    handle.readline()
    for line in handle:
        row = json.loads(line)
        for tag in row["tags"]:
            leaves[row["family"]][tag] += row["leaves"]
        groups[row["family"]] += row["leaves"]
print("## IND/COND/RET leaves joined", dict(groups))
for fam in ("IND", "COND", "RET"):
    print(fam, dict(leaves[fam].most_common(14)))
print("## alias groups", summary["groups"])
print("## alias classes", summary["classes"])
print("## alias cause", summary["causes"])

print("## shape ranking (tag, family leaves, functions touched, alone)")
for tag, v in sorted(rank["tag_stats"].items(), key=lambda x: (-x[1]["alone"], -x[1]["leaves"]))[:26]:
    print(f"| `{tag}` | {v['family']} | {v['leaves']} | {v['functions']} | {v['alone']} |")
print("## greedy")
for s in rank["greedy"][:14]:
    print(s["tag"], s["gain"], s["cumulative"])
print("## closure")
for label, v in rank["closure"].items():
    print(label, v["direct"], v["units_covered"], v["same-unit"], v["cross-unit"])
