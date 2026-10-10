#!/usr/bin/env python3
"""tally.py RANK.json CLASSIFICATION.json -- W45-CALLEE-CASCADE-RESIDUAL round 3 (from CG22-CALLBACK-REGISTRY-CONTRACT and its lineage): per
class of classification-before.json, the callees and the corpus functions
they decide alone (and with exactly one other callee) on RANK (rank.py's
output). Callees deciding fewer than 2 functions alone are not classified and
are summarised as one row."""
import collections
import json
import sys

rank = json.load(open(sys.argv[1]))
classes = json.load(open(sys.argv[2]))["callees"]
rows = collections.defaultdict(lambda: [0, 0, 0, []])
for row in rank["ranking"]:
    name = row["callee"]
    key = classes.get(name, ["unclassified (alone < 2)" if row["alone"] < 2 else "MISSING"])[0]
    entry = rows[key]
    entry[0] += 1
    entry[1] += row["alone"]
    entry[2] += row["pair"]
    if row["alone"] >= 2:
        entry[3].append("%s %d" % (name, row["alone"]))
print("population %d; decided alone by one callee %d" % (rank["population"], rank["alone_total"]))
for key in sorted(rows, key=lambda k: -rows[k][1]):
    callees, alone, pair, names = rows[key]
    print("%-28s callees %4d  alone %4d  pair %4d" % (key, callees, alone, pair))
    for name in names:
        print("      " + name)
