#!/usr/bin/env python3
"""Compact five-artifact summary of a `diff_artifacts.py` document.

Card AOM-DESTROY-SIBLINGS; card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED's
summariser, reused. Read-only.

    python3 -B docs/reports/aom-destroy-siblings/summarise_diff.py DIFF.json
"""
import json
import sys


def counts(node, key):
    return node.get(key, {}).get("counts", {})


def main() -> int:
    document = json.loads(open(sys.argv[1]).read())
    print(f"schema_version: {document.get('schema_version')}")
    print(f"archived: {document.get('archived')}")
    print(f"measured: {document.get('measured')}")
    for name, node in document["artifacts"].items():
        print(f"--- {name}")
        if not node.get("archived_present") or not node.get("measured_present"):
            print(f"    present: archived={node.get('archived_present')} "
                  f"measured={node.get('measured_present')}")
            continue
        for section in ("results", "backend_teardown", "units", "rows", "sources",
                        "functions", "libraries"):
            if section in node and "counts" in node[section]:
                c = node[section]["counts"]
                print(f"    {section}: {c.get('before')} -> {c.get('after')} "
                      f"(+{c.get('added')} -{c.get('removed')} ~{c.get('changed')})")
        for section in ("manifest", "audit_selection"):
            if section in node:
                for inner, value in node[section].items():
                    if isinstance(value, dict) and "counts" in value:
                        c = value["counts"]
                        print(f"    {section}.{inner}: {c.get('before')} -> {c.get('after')} "
                              f"(+{c.get('added')} -{c.get('removed')} ~{c.get('changed')})")
                    elif inner == "fields" and value:
                        print(f"    {section}.fields changed: {sorted(value)}")
        for key in ("envelope", "keys", "fields", "diagnostic_counts_before",
                    "diagnostic_counts_after", "scalars"):
            if node.get(key):
                text = json.dumps(node[key])
                print(f"    {key}: {text[:400]}")
        for section in ("sources",):
            if section in node:
                for direction in ("added", "removed", "changed"):
                    rows = node[section].get(direction) or []
                    if rows:
                        print(f"    {section}.{direction}: "
                              f"{[r if isinstance(r, str) else r.get('key') for r in rows]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
