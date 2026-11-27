#!/usr/bin/env python3
"""union_facts.py FACTS.json UNION -- RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: from
the member facts `member_facts_census` wrote, list per unit every store keyed
into UNION (`rd_kafka_op_s.rko_u.*`) that is open, every `?.NAME` store, and
whether the units' layouts of UNION agree (each differing entry named). A
reading aid."""
import collections
import json
import sys

facts = json.load(open(sys.argv[1]))
union = sys.argv[2]
layouts = {}
for unit, data in sorted(facts.items()):
    if "unread" in data:
        continue
    short = unit.split("/src/")[-1]
    for key, flows in data["stores"].items():
        opened = [flow.get("why") for flow in flows if flow["flow"] == "open"]
        if key.startswith(union) and opened:
            print("OPEN", short, key, opened)
        if key.startswith("?."):
            print("UNREAD-RECORD", short, key, flows)
    if union in data.get("union_layouts", {}):
        layouts[short] = data["union_layouts"][union]
print(len(layouts), "unit(s) give a layout of", union)
entries = collections.defaultdict(lambda: collections.defaultdict(list))
for short, layout in layouts.items():
    for part in ("paths", "records", "members"):
        for key, value in layout[part].items():
            entries[(part, key)][json.dumps(value, sort_keys=True)].append(short)
for (part, key), values in sorted(entries.items()):
    if len(values) > 1:
        print("DISAGREE", part, key, {value: units[:3] for value, units in values.items()})
for (part, key), values in sorted(entries.items()):
    if part == "paths" and key.startswith("leaders"):
        print("LEADERS", key, list(values))
