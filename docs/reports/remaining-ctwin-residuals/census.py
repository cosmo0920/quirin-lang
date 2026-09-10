#!/usr/bin/env python3
"""Census two 185-unit artifact sets, in the wave-16 consolidation's shape.

Card CTWIN-RESIDUALS. Read-only on its inputs.

    python3 -B census.py --before RUN_A --after RUN_B [--baseline DIR]
"""
from __future__ import annotations

import argparse
import collections
import gzip
import hashlib
import json
import pathlib

ARTIFACTS = {
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "contracts": "resource-diagnostics.contracts.json",
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "source-effects": "resource-diagnostics.source-effects.json",
}


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def load(run: pathlib.Path, name: str):
    return json.loads((run / ARTIFACTS[name]).read_bytes())


def units(report):
    return {f"{u['library']}/{u['file']}": u for u in report["units"]}


def unit_status(unit) -> str:
    return unit.get("status") or "data_only"


def leaves(unit):
    out = []
    for summary in unit.get("functions") or []:
        for diagnostic in summary.get("diagnostics") or []:
            out.append((summary["function"], diagnostic.get("line"),
                        diagnostic.get("category"), diagnostic.get("description")))
    return out


def kind_of(leaf) -> str:
    return leaf[2] or "?"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", required=True, type=pathlib.Path)
    parser.add_argument("--after", required=True, type=pathlib.Path)
    parser.add_argument("--baseline", type=pathlib.Path)
    arguments = parser.parse_args()

    print("== artifact digests")
    for name in sorted(ARTIFACTS):
        b = (arguments.before / ARTIFACTS[name]).read_bytes()
        a = (arguments.after / ARTIFACTS[name]).read_bytes()
        same = "IDENTICAL" if b == a else "DIFFERS"
        print(f"  {name:16} before {digest(b)[:12]}  after {digest(a)[:12]}  {same}")
        if arguments.baseline:
            path = arguments.baseline / f"{name}.json.gz"
            if path.exists():
                pinned = gzip.decompress(path.read_bytes())
                print(f"    vs pinned baseline: before "
                      f"{'IDENTICAL' if pinned == b else 'differs'}, after "
                      f"{'IDENTICAL' if pinned == a else 'differs'}")

    before = units(load(arguments.before, "diagnostics"))
    after = units(load(arguments.after, "diagnostics"))
    print(f"\n== units {len(before)} -> {len(after)}")
    print(f"units added: {sorted(set(after) - set(before))}")
    print(f"units removed: {sorted(set(before) - set(after))}")

    def tally(table):
        counts = collections.Counter(unit_status(u) for u in table.values())
        return dict(sorted(counts.items()))

    print(f"unit status before: {tally(before)}")
    print(f"unit status after : {tally(after)}")
    moves = [k for k in sorted(before)
             if k in after and unit_status(before[k]) != unit_status(after[k])]
    print(f"unit status moves: {len(moves)}")
    rmoves = [k for k in sorted(before) if k in after
              and before[k].get("reason_code") != after[k].get("reason_code")]
    print(f"unit reason_code moves: {len(rmoves)} {rmoves}")
    for key in moves:
        print(f"  {key}: {unit_status(before[key])} -> {unit_status(after[key])}")
    surfaced = [k for k in moves if unit_status(after[k]) == "Unsafe"]
    print(f"NEWLY SURFACED PROVISIONAL UNSAFE UNITS: {len(surfaced)} {surfaced}")
    promoted = [k for k in moves if unit_status(after[k]) == "Safe"]
    print(f"UNKNOWN->SAFE PROMOTIONS (must be 0): {len(promoted)} {promoted}")

    # Function-level status.
    def functions(table):
        out = {}
        for key, unit in table.items():
            for summary in unit.get("functions") or []:
                out[(key, summary["function"])] = summary.get("status")
        return out

    fb, fa = functions(before), functions(after)
    print(f"\nprojected functions: {len(fb)} -> {len(fa)}")
    fmoves = [k for k in sorted(fb) if k in fa and fb[k] != fa[k]]
    print(f"function status moves: {len(fmoves)}")
    for key in fmoves:
        print(f"  {key[0]}::{key[1]}: {fb[key]} -> {fa[key]}")

    # Diagnostic leaves.
    kb = collections.Counter()
    ka = collections.Counter()
    changed = {}
    for key in sorted(set(before) | set(after)):
        lb = leaves(before.get(key, {}))
        la = leaves(after.get(key, {}))
        kb.update(kind_of(x) for x in lb)
        ka.update(kind_of(x) for x in la)
        if collections.Counter(lb) != collections.Counter(la):
            changed[key] = (collections.Counter(lb), collections.Counter(la))
    print("\ncorpus diagnostic counts:")
    for kind in sorted(set(kb) | set(ka)):
        print(f"  {kind}: {kb[kind]} -> {ka[kind]} ({ka[kind] - kb[kind]:+d})")
    print(f"units with diagnostic changes: {len(changed)}")
    added = removed = 0
    for key, (cb, ca) in changed.items():
        plus = ca - cb
        minus = cb - ca
        added += sum(plus.values())
        removed += sum(minus.values())
        print(f"  UNIT {key}  -{sum(minus.values())} / +{sum(plus.values())}")
        for leaf, count in sorted(minus.items(), key=lambda x: str(x[0])):
            print(f"    -{count} {leaf[0]} L{leaf[1]} {leaf[2]}: {leaf[3]}")
        for leaf, count in sorted(plus.items(), key=lambda x: str(x[0])):
            print(f"    +{count} {leaf[0]} L{leaf[1]} {leaf[2]}: {leaf[3]}")
    print(f"\ntotal leaves: -{removed} / +{added}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
