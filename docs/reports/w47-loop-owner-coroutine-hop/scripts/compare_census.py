#!/usr/bin/env python3
"""Compare two thread-entry census runs site by site.

Usage: compare_census.py BEFORE.json AFTER.json [OUT.json]

A site is keyed by (file, line, function, callee); prints the verdict tally
of each side and every site whose verdict or reasons changed, with the AFTER
reasons or witness. Read-only.
"""

import collections
import json
import sys


def load(path):
    with open(path) as handle:
        return json.load(handle)


def key(site):
    return (site["file"], site["line"], site["function"], site["callee"])


def summary(site):
    result = site["result"]
    if result["verdict"] == "unresolved":
        return {"verdict": "unresolved", "reasons": result["reasons"],
                "reason_count": result["reason_count"]}
    if result["verdict"] == "not-discharged":
        return {"verdict": "not-discharged", "context": result["context"],
                "witness": result["witness"]}
    return {"verdict": "discharged", "contexts": result["contexts"]}


def main():
    before, after = load(sys.argv[1]), load(sys.argv[2])
    b = {key(s): summary(s) for s in before["sites"]}
    a = {key(s): summary(s) for s in after["sites"]}
    report = {
        "before": dict(collections.Counter(v["verdict"] for v in b.values())),
        "after": dict(collections.Counter(v["verdict"] for v in a.values())),
        "owners": {"before": before["owners"], "after": after["owners"]},
        "counts": {k: {"before": before.get(k), "after": after.get(k)}
                   for k in ("files", "functions", "edges", "unresolved_seeds",
                             "coroutine_bodies", "yields")},
        "moves": [],
        "witness_changes": 0,
        "only_before": [list(k) for k in sorted(set(b) - set(a))],
        "only_after": [list(k) for k in sorted(set(a) - set(b))],
    }
    for k in sorted(set(a) & set(b)):
        if b[k]["verdict"] != a[k]["verdict"] or (
                a[k]["verdict"] == "unresolved" and b[k] != a[k]):
            report["moves"].append({"site": list(k), "before": b[k], "after": a[k]})
        elif b[k] != a[k]:
            report["witness_changes"] += 1
    report["move_tally"] = dict(collections.Counter(
        f'{m["before"]["verdict"]} -> {m["after"]["verdict"]}' for m in report["moves"]))
    text = json.dumps(report, indent=1)
    if len(sys.argv) > 3:
        with open(sys.argv[3], "w") as handle:
            handle.write(text + "\n")
    print(text)


if __name__ == "__main__":
    main()
