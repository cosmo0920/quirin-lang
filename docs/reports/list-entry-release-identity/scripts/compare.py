#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""compare.py ROUTE BEFORE_JSON AFTER_JSON OUT_JSON -- LIST-ENTRY-RELEASE-IDENTITY.

Row-by-row comparison of two `resource-diagnostics.json` reports of one
route: per (unit file, function, occurrence) the status before and after and
the leaves (description at origin) lost and gained. Lists every row whose
status or leaves changed, the status totals of both sides and the moves.
A reading aid for the attribution; the arbiter is `quirin-audit measure`.
"""
import collections
import json
import sys


def rows(report):
    out = {}
    seen = collections.Counter()
    for unit in report["units"]:
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            seen[key] += 1
            leaves = collections.Counter(
                f"{d.get('origin')}: {d.get('description')}" for d in function.get("diagnostics", []) or [])
            out[(unit["file"], function["function"], seen[key])] = (function["status"], leaves)
    return out


def main():
    route, before_path, after_path, out_path = sys.argv[1:5]
    before = rows(json.load(open(before_path)))
    after = rows(json.load(open(after_path)))
    changed = []
    for key in sorted(set(before) | set(after)):
        b = before.get(key, ("absent", collections.Counter()))
        a = after.get(key, ("absent", collections.Counter()))
        if b == a:
            continue
        changed.append({
            "unit": key[0], "function": key[1], "occurrence": key[2],
            "before": b[0], "after": a[0],
            "lost": sorted((b[1] - a[1]).elements()), "gained": sorted((a[1] - b[1]).elements()),
        })
    totals = {side: dict(collections.Counter(status for status, _ in data.values()))
              for side, data in (("before", before), ("after", after))}
    moves = collections.Counter(f"{c['before']} -> {c['after']}" for c in changed if c["before"] != c["after"])
    out = {"route": route, "totals": totals, "moves": dict(moves),
           "rows_changed": len(changed), "rows": changed}
    json.dump(out, open(out_path, "w"), indent=1)
    print(route, totals, dict(moves), "rows changed", len(changed))
    for c in changed:
        mark = "MOVE " if c["before"] != c["after"] else "     "
        print(f"{mark}{c['unit']}::{c['function']}#{c['occurrence']} {c['before']} -> {c['after']} "
              f"lost {len(c['lost'])} gained {len(c['gained'])}")


if __name__ == "__main__":
    main()
