#!/usr/bin/env python3
"""HASH-REFINE: compare a fresh pinned-route measurement against the wave-14 baseline.

Byte comparison over all five artifacts, including `contracts`, which the
archived diff tool does not row-itemize (recorded blind spot
DIFF-TOOL-CONTRACTS). For `contracts` the comparison is repeated after
normalising the checkout path, because its map keys are absolute paths and this
run is from a worktree — the recorded worktree-provenance residue class.
"""
import gzip
import hashlib
import json
import pathlib
import sys
from collections import Counter

ARCHIVE = pathlib.Path("docs/reports/wave14-corpus/artifacts")
MEASURED = pathlib.Path(sys.argv[1])
WORKTREE = sys.argv[2] if len(sys.argv) > 2 else None
MAIN = sys.argv[3] if len(sys.argv) > 3 else "/home/cosmo/Documents/GitHub/qcore-c"

PAIRS = {
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "contracts": "resource-diagnostics.contracts.json",
}

status = {}
for stem, name in PAIRS.items():
    archived = ARCHIVE / f"{stem}.json.gz"
    measured = MEASURED / name
    if not archived.exists() or not measured.exists():
        print(f"{stem:16s} MISSING (archived={archived.exists()} measured={measured.exists()})")
        status[stem] = "missing"
        continue
    a = gzip.open(archived).read()
    m = measured.read_bytes()
    same = a == m
    status[stem] = "identical" if same else "differs"
    print(
        f"{stem:16s} {'IDENTICAL' if same else 'differs':10s} "
        f"archived={len(a)}B/{hashlib.sha256(a).hexdigest()[:12]} "
        f"measured={len(m)}B/{hashlib.sha256(m).hexdigest()[:12]}"
    )
    if not same and stem == "contracts" and WORKTREE:
        normalised = m.replace(WORKTREE.encode(), MAIN.encode())
        print(
            f"{'':16s} after path normalisation: "
            f"{'IDENTICAL' if normalised == a else 'STILL DIFFERS'}"
        )
        if normalised != a:
            aj, mj = json.loads(a), json.loads(normalised)
            for key in sorted(set(aj) | set(mj)):
                if aj.get(key) != mj.get(key):
                    print(f"{'':16s}   top-level key differs: {key}")

# Unit and function level comparison over the verdict-bearing artifact.
archived = json.loads(gzip.open(ARCHIVE / "diagnostics.json.gz").read())
measured = json.loads((MEASURED / PAIRS["diagnostics"]).read_bytes())


def key(unit):
    return (unit["library"], unit["file"])


A = {key(u): u for u in archived["units"]}
M = {key(u): u for u in measured["units"]}
print()
print(f"units archived={len(A)} measured={len(M)} added={sorted(M.keys() - A.keys())} "
      f"removed={sorted(A.keys() - M.keys())}")
moves = [(k, A[k]["status"], M[k]["status"]) for k in sorted(A.keys() & M.keys())
         if A[k]["status"] != M[k]["status"]]
print(f"unit verdict moves: {len(moves)}")
for move in moves:
    print("   ", move)
function_moves = 0
for k in sorted(A.keys() & M.keys()):
    a = {f["function"]: f["status"] for f in A[k]["functions"]}
    m = {f["function"]: f["status"] for f in M[k]["functions"]}
    for name in sorted(a.keys() & m.keys()):
        if a[name] != m[name]:
            function_moves += 1
            print("   fn move", k, name, a[name], "->", m[name])
print(f"function status moves: {function_moves}")
print()
for label, doc in (("archived", archived), ("measured", measured)):
    units = doc["units"]
    print(label, Counter(u["status"] for u in units),
          "src_fns=", sum(len(u["source_functions"]) for u in units),
          "projected=", sum(len(u["functions"]) for u in units),
          "leak=", doc["diagnostic_counts"].get("leak"),
          "unknown_alias=", doc["diagnostic_counts"].get("unknown_alias"),
          "unsupported=", doc["diagnostic_counts"].get("unsupported"))
