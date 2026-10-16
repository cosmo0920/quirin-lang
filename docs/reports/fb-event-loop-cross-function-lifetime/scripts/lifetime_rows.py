#!/usr/bin/env python3
"""FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME: lifetime_rows.py ROUTE REPORT OUT.json

Read-only. Over one `resource-diagnostics.json` (a file or the directory
holding it), list every function row with a leaf of the loop lifetime: rule
12t's `retention-undecided` / `retention-released-while-held` (by category or
description), or a leaf naming a loop operation or a registering function of
the census (`mk_event_*`, `flb_sched_timer_*`, collector operations). Each
row gets its status, its lifetime leaves, its other leaves, and a class:

* `L-only`   every non-Safe leaf is a lifetime leaf: a variant that decides
             them could move the row;
* `L+other`  lifetime leaves beside other leaves: at most a leaf trade;
* `other`    no lifetime leaf (not listed).

Also the route's status totals. Output: OUT.json; nothing is piped.
"""
import collections
import gzip
import hashlib
import json
import pathlib
import re
import sys

LIFETIME = re.compile(
    r"retention|retained|#holds|holds\(|\bmk_event_|flb_sched_timer|flb_input_collector|"
    r"collector_start|flb_input_set_collector|loop.owner|mk_event_loop")


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        gz = path / "resource-diagnostics.json.gz"
        path = gz if gz.exists() else path / "resource-diagnostics.json"
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rt") as handle:
        return json.load(handle), hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    route, report, out = sys.argv[1], sys.argv[2], sys.argv[3]
    data, digest = load(report)
    totals = collections.Counter()
    rows = []
    seen = collections.Counter()
    for unit in data["units"]:
        ordinal = seen[unit["file"]]
        seen[unit["file"]] += 1
        name = unit["file"] if ordinal == 0 else f"{unit['file']}#{ordinal}"
        for function in unit.get("functions", []):
            status = function.get("status")
            totals[status] += 1
            leaves = function.get("diagnostics", [])
            life, other = [], []
            for d in leaves:
                text = f"{d.get('category')}: {d.get('description')}"
                (life if LIFETIME.search(text) else other).append(
                    {"line": d.get("line"), "category": d.get("category"), "description": d.get("description")})
            if not life:
                continue
            rows.append({"unit": name, "function": function["function"], "status": status,
                         "class": "L-only" if not other else "L+other",
                         "lifetime_leaves": life, "other_leaves": other})
    rows.sort(key=lambda r: (r["class"], r["unit"], r["function"]))
    result = {"route": route, "report_sha256": digest, "totals": dict(totals),
              "rows": rows,
              "classes": dict(collections.Counter(r["class"] for r in rows))}
    with open(out, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=1, ensure_ascii=False)
        f.write("\n")
    print(route, digest[:8], dict(totals), result["classes"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
