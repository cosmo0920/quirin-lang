#!/usr/bin/env python3
"""Wave-16 consolidation: triangulate the composed diagnostics leaf set against
the two gateway cards' archived AFTER measurements.

Read-only, and reproducible from committed bytes: every input is an archived
artifact set. Each card measured its own AFTER without the other card's diff,
so the composed tree must differ from each card's AFTER by exactly the other
card's leaves. This asserts that in both directions rather than arguing it.

    python3 -B docs/reports/wave16-corpus/triangulate.py
"""
from __future__ import annotations

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]  # docs/reports/wave16-corpus/ -> repository root


def load_gz(path: pathlib.Path) -> dict:
    with gzip.open(path, "rt") as handle:
        return json.load(handle)


def load(path: pathlib.Path) -> dict:
    return json.loads(path.read_text())


def leaves(report: dict) -> collections.Counter:
    counter: collections.Counter = collections.Counter()
    for unit in report["units"]:
        key = f"{unit.get('library')}/{unit.get('file')}"
        for function in unit.get("functions", []):
            name = function.get("function") or function.get("name")
            for diagnostic in function.get("diagnostics", []):
                counter[(key, name, diagnostic.get("line"),
                         diagnostic.get("category"), diagnostic.get("description"))] += 1
    return counter


def statuses(report: dict) -> dict:
    return {f"{u.get('library')}/{u.get('file')}": u.get("status") for u in report["units"]}


def show(label: str, delta: collections.Counter) -> None:
    print(f"  {label}: {sum(v for v in delta.values() if v > 0)} added, "
          f"{-sum(v for v in delta.values() if v < 0)} removed")
    for leaf, count in sorted(delta.items(), key=lambda kv: (kv[0][0], kv[0][2] or 0)):
        if count:
            print(f"    {count:+d} {leaf[0]} {leaf[1]} L{leaf[2]} {leaf[3]}: {leaf[4]}")


def diff(before: collections.Counter, after: collections.Counter) -> collections.Counter:
    delta = collections.Counter(after)
    delta.subtract(before)
    return collections.Counter({k: v for k, v in delta.items() if v})


def main() -> int:
    baseline = load_gz(ROOT / "docs/reports/wave15-corpus/artifacts/diagnostics.json.gz")
    chained = load_gz(ROOT / "docs/reports/remaining-neighbours-chained-tail/artifacts/diagnostics.json.gz")
    alias = load_gz(ROOT / "docs/reports/remaining-alias-aggregate/artifacts/diagnostics.json.gz")
    composed = load_gz(ROOT / "docs/reports/wave16-corpus/artifacts/diagnostics.json.gz")

    lb, lc, la, lm = (leaves(r) for r in (baseline, chained, alias, composed))

    chained_own = diff(lb, lc)
    alias_own = diff(lb, la)
    composed_own = diff(lb, lm)

    print("CHAINED-TAIL archived AFTER vs wave-15 baseline")
    show("its own leaves", chained_own)
    print("ALIAS-AGGREGATE archived AFTER vs wave-15 baseline")
    show("its own leaves", alias_own)
    print("composed wave-16 measurement vs wave-15 baseline")
    show("measured leaves", composed_own)

    union = collections.Counter(chained_own)
    union.update(alias_own)
    union = collections.Counter({k: v for k, v in union.items() if v})

    print()
    print("TRIANGULATION")
    print("  union(CHAINED-TAIL, ALIAS-AGGREGATE) == composed measurement:",
          union == composed_own)
    unattributed = diff(union, composed_own)
    print("  unattributed leaves:", len(unattributed))
    for leaf, count in sorted(unattributed.items()):
        print(f"    UNATTRIBUTED {count:+d} {leaf}")

    print("  composed \\ CHAINED-TAIL == ALIAS-AGGREGATE's leaves:",
          diff(lc, lm) == alias_own)
    print("  composed \\ ALIAS-AGGREGATE == CHAINED-TAIL's leaves:",
          diff(la, lm) == chained_own)

    sb, sm = statuses(baseline), statuses(composed)
    moves = {k: (sb[k], sm[k]) for k in sb if sb.get(k) != sm.get(k)}
    print("  unit status moves:", len(moves), moves or "")
    print("  newly surfaced provisional Unsafe:",
          [k for k, (b, a) in moves.items() if a == "Unsafe"])
    return 0 if not unattributed and not moves else 1


if __name__ == "__main__":
    sys.exit(main())
