#!/usr/bin/env python3
"""union_places.py FACTS.json UNION ARM... -- UNION-OPAQUE-RECORD-WRITES-BY-TYPE: from the
member facts `member_facts_census` wrote, for each ARM of UNION
(`rd_kafka_op_s.rko_u.*`): its byte range in every unit's layout (and whether
the units agree), every store keyed into UNION whose arm range overlaps it
(with its flows, per unit), every store into the whole union, and every record
the union holds whose placements overlap it with the units that open that
record by type (`opened`). A reading aid for the census."""
import collections
import json
import sys

facts = json.load(open(sys.argv[1]))
union = sys.argv[2]
arms = sys.argv[3:]


def overlap(a, b):
    return a[0] < b[0] + b[1] and b[0] < a[0] + a[1]


layouts = {}
for unit, data in facts.items():
    if "unread" not in data and union in data.get("union_layouts", {}):
        layouts[unit.split("/src/")[-1]] = data["union_layouts"][union]
paths = collections.defaultdict(set)
records = collections.defaultdict(set)
for layout in layouts.values():
    for arm, rng in layout["paths"].items():
        paths[arm].add(tuple(rng))
    for record, placements in layout["records"].items():
        records[record].add(None if placements is None else tuple(map(tuple, placements)))
out = {"units_with_layout": sorted(layouts), "arms": {}}
for arm in arms:
    ranges = paths.get(arm, set())
    entry = {"range": sorted(ranges), "agree": len(ranges) == 1, "stores": [], "held": []}
    rng = next(iter(ranges)) if len(ranges) == 1 else None
    for unit, data in sorted(facts.items()):
        if "unread" in data:
            continue
        short = unit.split("/src/")[-1]
        for key, flows in data["stores"].items():
            if not key.startswith(union):
                continue
            store_arm = key.split("@", 1)[1] if "@" in key else None
            store_ranges = paths.get(store_arm) if store_arm else None
            if store_arm is None or rng is None or not store_ranges or len(store_ranges) != 1 \
                    or overlap(rng, next(iter(store_ranges))) or store_arm == arm:
                entry["stores"].append({"unit": short, "key": key,
                                        "range": sorted(store_ranges) if store_ranges else None,
                                        "flows": flows})
    for record, placements in sorted(records.items()):
        meets = rng is None or any(p is None for p in placements) or any(
            overlap(rng, q) for p in placements for q in p)
        if not meets:
            continue
        openers = sorted(unit.split("/src/")[-1] for unit, data in facts.items()
                         if "unread" not in data and record in data.get("opened", {}))
        entry["held"].append({"record": record, "placements": sorted(map(list, placements)),
                              "opened_in": openers})
    out["arms"][arm] = entry
json.dump(out, sys.stdout, indent=1)
print()
