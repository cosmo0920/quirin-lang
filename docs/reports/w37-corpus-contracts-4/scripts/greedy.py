#!/usr/bin/env python3
"""Card W37-CORPUS-CONTRACTS-4: greedy set cover over the blocked set.

A function moves only when EVERY one of its blockers is declared, so choosing
by sole-blocker count alone under-counts a set: two callees declared together
can free a function neither frees alone. This computes the greedy cover and
prints, for a named set, exactly which functions become leaf-free.

Usage:
  python3 -B measure/w37/greedy.py <dir> --depth 20
  python3 -B measure/w37/greedy.py <dir> --set NAME,NAME,...
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
                out.append((u["library"], u["file"], f["function"], frozenset(names)))
    return out


def main() -> int:
    args = sys.argv[1:]
    where = pathlib.Path(args[0])
    depth = 20
    chosen = None
    for i, a in enumerate(args):
        if a == "--depth":
            depth = int(args[i + 1])
        if a == "--set":
            chosen = [n for n in args[i + 1].split(",") if n]
    data = load(where)
    units = data["units"]
    blocked = blocked_set(units)
    leafcount: collections.Counter = collections.Counter()
    for u in units:
        for f in u.get("functions", []):
            for d in f.get("diagnostics", []):
                m = PAT.match(d.get("description") or "")
                if m:
                    leafcount[m.group(1)] += 1

    if chosen is None:
        print(f"=== greedy cover, depth {depth} (blocked set {len(blocked)}) ===")
        have: set = set()
        freed_so_far = 0
        for step in range(depth):
            gain: collections.Counter = collections.Counter()
            for lib, file, fn, names in blocked:
                if names <= have:
                    continue
                missing = names - have
                if len(missing) == 1:
                    gain[next(iter(missing))] += 1
            if not gain:
                print("  (no further single-step gain)")
                break
            # tie-break by leaf mass, then by name, so the run is deterministic
            best = max(gain.items(), key=lambda kv: (kv[1], leafcount[kv[0]], kv[0]))
            have.add(best[0])
            freed_so_far += best[1]
            print(f"  {step + 1:2d}. +{best[1]:2d} funcs  cum={freed_so_far:3d}  "
                  f"{best[0]:40s} leaves={leafcount[best[0]]}")
        return 0

    have = set(chosen)
    freed = [b for b in blocked if b[3] <= have]
    print(f"=== the named set of {len(chosen)} callees ===")
    for n in chosen:
        sole = [b for b in blocked if b[3] == {n}]
        print(f"  {n:40s} sole_blocker_of={len(sole):2d} leaves={leafcount[n]}")
    print(f"  leaves decided corpus-wide (UPPER BOUND): {sum(leafcount[n] for n in chosen)}")
    print()
    print(f"=== functions that become LEAF-FREE (every blocker declared): {len(freed)} ===")
    for lib, file, fn, names in sorted(freed):
        extra = "" if len(names) == 1 else f"   [needs all of: {sorted(names)}]"
        print(f"  {lib}/{file} :: {fn}{extra}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
