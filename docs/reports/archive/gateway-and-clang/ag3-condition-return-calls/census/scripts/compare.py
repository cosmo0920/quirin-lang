"""Compare two resource-diagnostics.json reports: status moves and leaf deltas.

Usage: compare.py BASE.json TIP.json [OUT.json]
Leaves are (file, function, category, description, line) multisets.
"""
import json, re, sys
from collections import Counter

def load(path):
    d = json.load(open(path))
    rows, leaves = {}, Counter()
    for u in d["units"]:
        for f in u.get("functions", []):
            key = (u.get("file"), f["function"])
            rows[key] = f["status"]
            for x in f.get("diagnostics", []):
                leaves[(u.get("file"), f["function"], x["category"], x["description"], x.get("line"))] += 1
    return rows, leaves

def family(desc):
    desc = re.sub(r"`[^`]*`", "X", desc)
    return desc[:100]

b_rows, b_leaves = load(sys.argv[1])
t_rows, t_leaves = load(sys.argv[2])
moves = Counter()
moved = []
for k in sorted(set(b_rows) | set(t_rows), key=str):
    a, b = b_rows.get(k), t_rows.get(k)
    if a != b:
        moves[(a, b)] += 1
        moved.append({"file": k[0], "function": k[1], "base": a, "tip": b})
removed = b_leaves - t_leaves
added = t_leaves - b_leaves
print("status totals base", Counter(b_rows.values()), "tip", Counter(t_rows.values()))
print("moves", dict(moves))
for m in moved:
    print("  move", m)
print("removed leaves", sum(removed.values()), "added", sum(added.values()))
fr, fa = Counter(), Counter()
for (f, fn, c, d, l), n in removed.items():
    fr[(c, family(d))] += n
for (f, fn, c, d, l), n in added.items():
    fa[(c, family(d))] += n
print("removed by family:")
for k, n in fr.most_common(30):
    print(f"  {n:6d} {k}")
print("added by family:")
for k, n in fa.most_common(30):
    print(f"  {n:6d} {k}")
if len(sys.argv) > 3:
    json.dump({"moves": moved,
               "removed": [list(k) + [n] for k, n in sorted(removed.items(), key=str)],
               "added": [list(k) + [n] for k, n in sorted(added.items(), key=str)]},
              open(sys.argv[3], "w"), indent=0)
