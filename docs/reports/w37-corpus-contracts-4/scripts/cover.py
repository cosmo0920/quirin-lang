#!/usr/bin/env python3
"""Card W37-CORPUS-CONTRACTS-4: re-derive the cover from a BEFORE/baseline run.

Read-only. Prints the blocked set, the sole-blocker histogram and the
function-cover yield ranking, so the choice of callees is a measurement rather
than a carry-forward from the predecessor.

Usage: python3 -B measure/w37/cover.py <run-or-archive-dir> [--callee NAME ...]
"""
from __future__ import annotations

import collections
import gzip
import json
import pathlib
import re
import sys

PAT = re.compile(r"^call to uncontracted function `([^`]+)`")


def load(where: pathlib.Path) -> dict:
    gz = where / "diagnostics.json.gz"
    if gz.exists():
        return json.loads(gzip.decompress(gz.read_bytes()))
    return json.loads((where / "resource-diagnostics.json").read_bytes())


def blocked_set(units):
    """Functions every one of whose leaves is an uncontracted-call leaf."""
    out = []
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
                out.append((u["library"], u["file"], f["function"], f["status"], names))
    return out


def main() -> int:
    args = sys.argv[1:]
    where = pathlib.Path(args[0])
    wanted = [args[i + 1] for i, a in enumerate(args) if a == "--callee"]
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

    blocked = blocked_set(units)
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
    leafcount: collections.Counter = collections.Counter()
    for u in units:
        for f in u.get("functions", []):
            for d in f.get("diagnostics", []):
                m = PAT.match(d.get("description") or "")
                if m:
                    leafcount[m.group(1)] += 1

    print()
    print("    top callees by SOLE-BLOCKER count (the function-cover yield):")
    for n, v in singles.most_common(30):
        print(f"      {v:3d}  {n:40s} named_by={naming[n]:4d} leaves={leafcount[n]}")
    print(f"      total functions with a single blocker: {sum(singles.values())}")
    print()
    print("    top callees by LEAF mass (deliberately NOT the selection key):")
    for n, v in leafcount.most_common(15):
        print(f"      {v:5d}  {n:40s} sole_blocker_of={singles[n]}")

    if wanted:
        print()
        print("=== the candidate callees named on the command line ===")
        total = 0
        for name in wanted:
            freed = [b for b in blocked if b[4] == {name}]
            total += len(freed)
            print(f"  {name}: sole blocker of {len(freed)}; named by {naming[name]}; "
                  f"{leafcount[name]} leaves corpus-wide")
            for lib, file, fn, status, _ in sorted(freed):
                n_leaves = 0
                for u in units:
                    if u["library"] == lib and u["file"] == file:
                        for f in u.get("functions", []):
                            if f["function"] == fn:
                                n_leaves = len(f["diagnostics"])
                print(f"      {lib}/{file} :: {fn}  status={status} leaves={n_leaves}")
        print(f"  PREDICTED functions whose last blocker is removed (UPPER BOUND): {total}")
        leaf_total = sum(leafcount[n] for n in wanted)
        print(f"  PREDICTED uncontracted-call leaves decided, corpus-wide: {leaf_total}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
