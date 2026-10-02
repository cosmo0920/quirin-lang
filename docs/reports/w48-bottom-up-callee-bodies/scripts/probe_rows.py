#!/usr/bin/env python3
"""W48-BOTTOM-UP-CALLEE-BODIES: the `mk_vhost_set` acceptance probe's rows.

Usage: probe_rows.py REPORT.json [REPORT.json...]

Prints `mk_vhost_set`'s and `mk_vhost_set_property`'s status and leaves from
each contracts-only report the round-12 probe (`tip-contracts.sh`) writes.
"""
import json
import sys

WANTED = ("mk_vhost_set", "mk_vhost_set_property")


def walk(node):
    if isinstance(node, dict):
        if "function" in node and "status" in node:
            yield node
        for value in node.values():
            yield from walk(value)
    elif isinstance(node, list):
        for value in node:
            yield from walk(value)


def main() -> int:
    for path in sys.argv[1:]:
        print(f"== {path}")
        seen = set()
        for row in walk(json.load(open(path))):
            if row["function"] not in WANTED or row["function"] in seen:
                continue
            seen.add(row["function"])
            print(f"  {row['function']}: {row['status']}")
            for leaf in row.get("diagnostics") or []:
                print(f"     {leaf.get('line')} {leaf.get('category')}: {leaf.get('description')}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
