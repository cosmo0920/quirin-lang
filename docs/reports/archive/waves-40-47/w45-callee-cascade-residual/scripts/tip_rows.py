#!/usr/bin/env python3
"""W45-CALLEE-CASCADE-RESIDUAL (from CG10-INDIRECT-ACCESS-SOLE-BLOCKERS): `mk_vhost_set` and `mk_vhost_set_property` rows of the
tip probe's reports (usage: tip_rows.py REPORT.json ...)."""

import json
import sys

WANTED = ("mk_vhost_set", "mk_vhost_set_property")


def walk(node, label):
    if isinstance(node, dict):
        if node.get("function") in WANTED and "status" in node:
            print(label, node["function"], node.get("status"))
            for leaf in node.get("diagnostics") or []:
                print("   ", leaf.get("origin") or leaf.get("line"), leaf.get("description"))
        for value in node.values():
            walk(value, label)
    elif isinstance(node, list):
        for value in node:
            walk(value, label)


for path in sys.argv[1:]:
    walk(json.load(open(path)), path.rsplit("/", 1)[-1])
