#!/usr/bin/env python3
"""The tip candidate map against FLUENT-BIT-LINK-PREPARE's, and the 24 Monkey
callees against BODY-SUMMARY-REUSE's experiment homes.

usage: compare_maps.py BASE_MAP_GZ BASE_REFUSALS_GZ TIP_MAP TIP_REFUSALS EXPERIMENT_MAP COVERAGE OUT

BASE_* are the committed `link-map.candidate*.json.gz` of
docs/reports/fluent-bit-link-prepare (the same user database). TIP_* are this
card's run. It reports every name whose decision or home changed, by kind,
and checks that each of the 24 refused census callees now derives exactly
the experiment's home with `monkey-libevent/` read as `monkey/` (the same
file under the linked configuration's label). Auxiliary; decides nothing.
"""
import collections
import gzip
import json
import sys

base_map, base_refusals, tip_map, tip_refusals, experiment, coverage, out = sys.argv[1:8]
load = lambda path: json.load(gzip.open(path) if path.endswith(".gz") else open(path))
base = {s["function"]: s["home_unit"] for s in load(base_map)["symbols"]}
tip = {s["function"]: s["home_unit"] for s in load(tip_map)["symbols"]}
base_refused = {r["function"]: r for r in load(base_refusals)["refused"]}
tip_refused = {r["function"]: r for r in load(tip_refusals)["refused"]}
experiment_home = {s["function"]: s["home_unit"] for s in load(experiment)["symbols"]}
coverage = load(coverage)

changes = collections.Counter()
newly_derived = {}
for name in sorted(set(base) | set(tip) | set(base_refused) | set(tip_refused)):
    before = ("derived", base[name]) if name in base else ("refused", base_refused.get(name, {}).get("reason"))
    after = ("derived", tip[name]) if name in tip else ("refused", tip_refused.get(name, {}).get("reason"))
    if before == after:
        continue
    kind = f"{before[0]}:{before[1] if before[0] == 'refused' else 'home'} -> {after[0]}"
    if before[0] == "refused" and after[0] == "derived":
        newly_derived[name] = after[1]
        kind += " monkey/" if after[1].startswith("monkey/") else " other"
    changes[kind] += 1

census_monkey = sorted(
    name
    for name, outcome in coverage["callees"].items()
    if outcome["outcome"].startswith("derived, home differs")
) if isinstance(coverage.get("callees"), dict) else None
relabelled = {}
for name, home in experiment_home.items():
    if home.startswith("monkey-libevent/") and name in tip:
        relabelled[name] = tip[name] == "monkey/" + home[len("monkey-libevent/"):]
result = {
    "base_derived": len(base),
    "tip_derived": len(tip),
    "changes": dict(changes),
    "newly_derived_all_monkey": all(home.startswith("monkey/") for home in newly_derived.values()),
    "newly_derived": newly_derived,
    "experiment_monkey_homes": len(relabelled),
    "experiment_monkey_homes_equal_after_relabel": all(relabelled.values()) and bool(relabelled),
    "experiment_monkey_homes_detail": relabelled,
}
json.dump(result, open(out, "w"), indent=1, sort_keys=True)
print(json.dumps({k: v for k, v in result.items() if k not in ("newly_derived", "experiment_monkey_homes_detail")}, indent=1))
