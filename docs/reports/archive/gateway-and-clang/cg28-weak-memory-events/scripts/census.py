#!/usr/bin/env python3
"""CG28-WEAK-MEMORY-EVENTS (from CG19-ATOMIC-SOURCE-ROUTE): census of the atomic leaves of one run.

Usage: census.py RUN [--json OUT]   (a run directory or a diagnostics file)

Lists every unsupported leaf whose description names an atomic builtin
(`__atomic_*`, `__c11_atomic_*`, `__sync_*`) -- as an uncontracted call
(BEFORE) or as the atomic route's named refusal (AFTER) -- with the function,
its status, and whether the function's every leaf is one of them (a function
the route alone could free).
"""

import argparse
import collections
import json
import pathlib
import re

from compare import load

ATOMIC = re.compile(r"`(__atomic_[A-Za-z0-9_]*|__c11_atomic_[A-Za-z0-9_]*|__sync_[A-Za-z0-9_]*|_Atomic [a-z ]+)`")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    _, functions = load(args.run)
    rows = []
    by_builtin = collections.Counter()
    by_description = collections.Counter()
    for (unit, function), (status, leaves) in sorted(functions.items()):
        atomic = {key: count for key, count in leaves.items()
                  if key[1] and ATOMIC.search(key[1])}
        if not atomic:
            continue
        for (_, description), count in atomic.items():
            by_builtin[ATOMIC.search(description).group(1)] += count
            by_description[description] += count
        rows.append({
            "unit": unit,
            "function": function,
            "status": status,
            "atomic_leaves": {d: c for (_, d), c in atomic.items()},
            "only_atomic_leaves": sum(leaves.values()) == sum(atomic.values()),
        })
    record = {
        "functions": len(rows),
        "leaves": sum(by_builtin.values()),
        "by_builtin": dict(by_builtin.most_common()),
        "by_description": dict(by_description.most_common()),
        "functions_with_only_atomic_leaves": [
            f"{row['unit']}::{row['function']}" for row in rows if row["only_atomic_leaves"]
        ],
        "rows": rows,
    }
    print(json.dumps({k: v for k, v in record.items() if k != "rows"}, indent=1, ensure_ascii=False))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
