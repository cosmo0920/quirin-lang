#!/usr/bin/env python3
"""Compact per-unit census table from census.json and simulation.json."""
import json, os, sys, collections

OUT = os.path.dirname(os.path.abspath(__file__))
dest = sys.argv[1]
c = json.load(open(f"{OUT}/census.json"))
s = {o["unit"]: o for o in json.load(open(f"{OUT}/simulation.json"))}
rows = []
for r in c["rows"]:
    if not r["preprocessed"]:
        rows.append(dict(unit=r["unit"], preprocessed=False))
        continue
    o = s[r["unit"]]
    line = r["run_line"] or {}
    rows.append(dict(
        unit=r["unit"],
        run_reachable=line.get("reachable"), run_named_only=line.get("named_only"),
        run_held_after_rule40=line.get("held"), run_exhausted=line.get("exhausted"),
        reachable=r["reachable"], named_only=r["named_only"], stray=r["stray"],
        reachable_homes=[h for h in r["reachable_homes"] if h not in set(r["stray"])],
        reachable_without_stray=r["after"], held_after=o["held_after"], withheld_after=o["withheld"],
        sole_blocked_functions=r["sole_blocked_functions"],
        co_blocked_functions=len(r["co_blocked_functions"]),
        candidate_movers=o["candidate_movers"],
    ))
json.dump(dict(budget=24, named_homes=c["named"], run_homes=c["homes"],
               scope_mismatches_vs_run=len(c["mismatch"]), rows=rows),
          open(dest, "w"), indent=1)
pre = [r for r in rows if r.get("reachable") is not None]
print("units", len(rows), "reproduced", len(pre))
print("exhausted at run", sum(1 for r in pre if r["run_exhausted"]))
print("over budget without strays", sum(1 for r in pre if r["reachable_without_stray"] > 24))
print("stray probes (all units)", sum(len(r["stray"]) for r in pre))
print("stray by name", collections.Counter(n for r in pre for n in r["stray"]).most_common(30))
print("sole-blocked functions", sum(len(r["sole_blocked_functions"]) for r in pre))
print("co-blocked functions", sum(r["co_blocked_functions"] for r in pre))
dist = collections.Counter(r["reachable_without_stray"] for r in pre)
print("reachable without strays distribution", sorted(dist.items()))
