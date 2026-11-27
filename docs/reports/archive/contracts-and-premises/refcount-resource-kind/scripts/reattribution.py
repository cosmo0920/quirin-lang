#!/usr/bin/env python3
"""reattribution.py BEFORE AFTER -- card REFCOUNT-RESOURCE-KIND: the
`unresolved resource identity` leaves AFTER adds at the source line where
BEFORE read `call to uncontracted function `rd_kafka_topic_partition_list_destroy``
(the list a now-declared destructor releases came from an undeclared
constructor), and the ones added elsewhere. Prints functions, leaves, others."""
import json
import sys


def load(path):
    rows = {}
    for unit in json.load(open(path))["units"]:
        for function in unit["functions"]:
            rows[(unit["file"], function["function"])] = [
                (d.get("origin"), d["description"]) for d in function.get("diagnostics") or []]
    return rows


before, after = load(sys.argv[1]), load(sys.argv[2])
functions = leaves = others = 0
NAME = "uncontracted function `rd_kafka_topic_partition_list_destroy`"
for key, now in after.items():
    if key not in before:
        continue
    removed = {origin for origin, text in before[key] if NAME in text}
    added = [(origin, text) for origin, text in now
             if text.startswith("unresolved resource identity") and (origin, text) not in before[key]]
    hits = [leaf for leaf in added if leaf[0] in removed]
    if hits:
        functions += 1
        leaves += len(hits)
    others += len(added) - len(hits)
print("functions %d leaves %d elsewhere %d" % (functions, leaves, others))
