#!/usr/bin/env python3
"""Why a corpus callee's own body yields no validated summary.

Usage: body_reasons.py DIAGNOSTICS.json[.gz] CALLEE_CENSUS.json OUT.json

For every callee of `callee_census.py`'s `same-unit-selected:*` and
`other-unit:*` classes, reads the callee's own analysed row(s) and sorts its
diagnostics into blocker families. A callee is

* `cascade-only` when its own body is blocked only by `call to uncontracted
  function` leaves: it would be derivable exactly when those callees are;
* `destructor-shaped` when its name reads as a release (`destroy`, `free`,
  `release`, `unref`, `clear`, `term`, `fini`) -- the population a
  releases-its-formal summary would target. It is sized, not admitted.

Each family count is the number of callees whose body carries it, and the
leaves column is the callers' uncontracted-call leaves those callees account
for. A callee defined in several units takes the union of its rows.
"""

import collections
import gzip
import json
import re
import sys

FAMILIES = [
    ("uncontracted-call", re.compile(r"^call to uncontracted function")),
    ("indirect-access", re.compile(r"^indirect memory access")),
    ("condition", re.compile(r"in (branch condition|for-loop condition|while condition|do-while condition|switch expression)")),
    ("unknown-alias", re.compile(r"^unresolved resource identity")),
    ("return-expression", re.compile(r"in return expression")),
    ("braced-or-declaration", re.compile(r"braced initializer|declaration")),
    ("call-argument", re.compile(r"call has an unmodeled|argument")),
    ("assignment", re.compile(r"assignment")),
]
DESTRUCTOR = re.compile(r"(destroy|free|release|unref|clear|_term|fini|dtor|cleanup)", re.I)


def family(description):
    for name, pattern in FAMILIES:
        if pattern.search(description):
            return name
    return "other"


def main() -> int:
    diagnostics, census_path, out = sys.argv[1:4]
    opener = gzip.open if diagnostics.endswith(".gz") else open
    data = json.load(opener(diagnostics))
    census = json.load(open(census_path))
    bodies = collections.defaultdict(list)
    for unit in data["units"]:
        for entry in unit["functions"]:
            bodies[entry["function"]].append(entry)
    table = collections.defaultdict(lambda: collections.Counter())
    table_leaves = collections.defaultdict(lambda: collections.Counter())
    rows = []
    for row in census["callees"]:
        classes = [c for c in row["classes"] if c.startswith(("same-unit-selected", "other-unit"))]
        if not classes:
            continue
        leaves = sum(row["classes"][c] for c in classes)
        families = collections.Counter()
        statuses = set()
        for entry in bodies.get(row["callee"], []):
            statuses.add(entry["status"])
            for d in entry.get("diagnostics") or []:
                families[family(d["description"])] += 1
        kinds = set(families)
        cascade_only = kinds == {"uncontracted-call"}
        destructor = bool(DESTRUCTOR.search(row["callee"]))
        group = "/".join(sorted({c.split(":")[0] for c in classes}))
        for kind in kinds:
            table[group][kind] += 1
            table_leaves[group][kind] += leaves
        table[group]["(callees)"] += 1
        table_leaves[group]["(callees)"] += leaves
        if cascade_only:
            table[group]["(cascade-only)"] += 1
            table_leaves[group]["(cascade-only)"] += leaves
        if destructor:
            table[group]["(destructor-shaped)"] += 1
            table_leaves[group]["(destructor-shaped)"] += leaves
            if cascade_only:
                table[group]["(destructor-shaped, cascade-only)"] += 1
                table_leaves[group]["(destructor-shaped, cascade-only)"] += leaves
        rows.append({
            "callee": row["callee"], "leaves": leaves, "sole": row["sole"],
            "statuses": sorted(statuses), "families": dict(families.most_common()),
            "cascade_only": cascade_only, "destructor_shaped": destructor,
        })
    summary = {
        group: {kind: {"callees": table[group][kind], "leaves": table_leaves[group][kind]}
                for kind in sorted(table[group], key=lambda k: -table_leaves[group][k])}
        for group in sorted(table)
    }
    with open(out, "w") as handle:
        json.dump({"summary": summary, "callees": rows}, handle, indent=1)
        handle.write("\n")
    for group, kinds in summary.items():
        print("==", group)
        for kind, value in kinds.items():
            print(f"   {kind:36} callees {value['callees']:5} leaves {value['leaves']:6}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
