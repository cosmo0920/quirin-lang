#!/usr/bin/env python3
"""Card W36-CORPUS-CONTRACTS-3: the ex-ante prediction, from BEFORE only.

Read-only. Computed from the BEFORE run's own diagnostics — which reproduce
`docs/reports/joined-provenance-cycles/artifacts` byte for byte on both
verdict-bearing artifacts — and written before the AFTER run finished, so the
predicted-vs-measured comparison in the report is a real one.

Usage: python3 -B measure/w36/predict.py <run-or-archive-dir>
"""
from __future__ import annotations

import collections
import gzip
import json
import pathlib
import re
import sys

PAT = re.compile(r"^call to uncontracted function `([^`]+)`")

# The six callees this card declares, and the two families it declines.
DECLARED = (
    "cfl_sds_destroy",
    "rd_kafka_mem_free",
    "cmt_destroy",
    "cJSON_Delete",
    "mk_print",
    "ctr_destroy",
)
DECLINED = ("add", "memcpy", "memset")


def load(where: pathlib.Path) -> dict:
    gz = where / "diagnostics.json.gz"
    if gz.exists():
        return json.loads(gzip.decompress(gz.read_bytes()))
    return json.loads((where / "resource-diagnostics.json").read_bytes())


def main() -> int:
    where = pathlib.Path(sys.argv[1])
    data = load(where)
    units = data["units"]
    print(f"source: {where}")
    print(f"units: {len(units)}")

    uhist = collections.Counter(u.get("status") for u in units)
    print(f"unit status: {dict(sorted(uhist.items()))}")
    functions = [(u, f) for u in units for f in u.get("functions", [])]
    fhist = collections.Counter(f.get("status") for _, f in functions)
    print(f"projected functions: {len(functions)}  {dict(sorted(fhist.items()))}")
    counts: collections.Counter = collections.Counter()
    for u in units:
        counts.update(u.get("diagnostic_counts", {}))
    print("corpus diagnostic counts:")
    for category in sorted(counts):
        print(f"  {category}: {counts[category]}")

    # --- the blocked set: functions whose EVERY leaf is an uncontracted call
    blocked = []
    for u in units:
        for f in u.get("functions", []):
            leaves = f.get("diagnostics", [])
            if not leaves:
                continue
            names, only = set(), True
            for d in leaves:
                m = PAT.match(d.get("description") or "")
                if m:
                    names.add(m.group(1))
                else:
                    only = False
                    break
            if only and names:
                blocked.append((u["library"], u["file"], f["function"], f["status"], names))

    print()
    print(f"=== functions blocked ONLY by uncontracted-call leaves: {len(blocked)} ===")
    print(f"    status histogram: {dict(collections.Counter(b[3] for b in blocked))}")
    sizes = collections.Counter(len(b[4]) for b in blocked)
    print(f"    distinct-callee-count histogram: {dict(sorted(sizes.items()))}")

    naming: collections.Counter = collections.Counter()
    singles: collections.Counter = collections.Counter()
    for *_, names in blocked:
        for n in names:
            naming[n] += 1
        if len(names) == 1:
            singles[next(iter(names))] += 1
    print()
    print("    top callees by BLOCKED-FUNCTION naming count:")
    for n, v in naming.most_common(20):
        print(f"      {v:3d}  {n}")
    print()
    print("    top callees by SOLE-BLOCKER count (the function-cover yield):")
    for n, v in singles.most_common(20):
        print(f"      {v:3d}  {n}")
    print(f"      total functions with a single blocker: {sum(singles.values())}")

    # --- this card's prediction
    print()
    print("=== PREDICTION: the six declared callees ===")
    total = 0
    for name in DECLARED:
        freed = [b for b in blocked if b[4] == {name}]
        total += len(freed)
        print(f"  {name}: sole blocker of {len(freed)} function(s); named by {naming[name]}")
        for lib, file, fn, status, _ in sorted(freed):
            leaves = 0
            for u in units:
                if u["library"] == lib and u["file"] == file:
                    for f in u.get("functions", []):
                        if f["function"] == fn:
                            leaves = len(f["diagnostics"])
            print(f"      {lib}/{file} :: {fn}  status={status} leaves={leaves}")
    print(f"  PREDICTED functions whose last blocker is removed: {total}")

    leaf_total = 0
    for u in units:
        for f in u.get("functions", []):
            for d in f.get("diagnostics", []):
                m = PAT.match(d.get("description") or "")
                if m and m.group(1) in DECLARED:
                    leaf_total += 1
    print(f"  PREDICTED uncontracted-call leaves named by the six, corpus-wide: {leaf_total}")
    print("  (a leaf is DECIDED by the declaration; a function only moves when")
    print("   the declaration was its LAST blocker, which is why the two differ)")

    print()
    print("=== the declined callees, for the record ===")
    for name in DECLINED:
        freed = [b for b in blocked if b[4] == {name}]
        leaves = 0
        for u in units:
            for f in u.get("functions", []):
                for d in f.get("diagnostics", []):
                    m = PAT.match(d.get("description") or "")
                    if m and m.group(1) == name:
                        leaves += 1
        print(f"  {name}: sole blocker of {len(freed)}, named by {naming[name]}, {leaves} leaves")
    return 0


if __name__ == "__main__":
    sys.exit(main())
