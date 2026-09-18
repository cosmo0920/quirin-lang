#!/usr/bin/env python3
"""Card W37-CORPUS-CONTRACTS-4: attribute every moved leaf, nothing summarised.

Groups the BEFORE/AFTER leaf delta into the classes the report publishes, and
in particular separates a genuine ADDITION from a same-site TEXT SUBSTITUTION,
which a naive added/removed count cannot tell apart.
"""
from __future__ import annotations

import collections
import json
import pathlib
import re
import sys

PAT = re.compile(r"^call to uncontracted function `([^`]+)`")
WITHDRAWN = re.compile(r"^call to `([^`]+)` whose declared contract was withdrawn")
DECLARED = {
    "rd_kafka_topic_partition_list_sort",
    "mpack_write_cstr", "mpack_write_str", "mpack_write_i64", "mpack_write_double",
    "cmt_metric_get_timestamp", "cmt_metric_set_double",
    "cmt_mpack_consume_uint_tag", "cfl_time_now", "LZ4_resetStream",
}


def load(where: pathlib.Path) -> dict:
    return json.loads((where / "resource-diagnostics.json").read_bytes())


def leaves(data):
    out = {}
    for u in data["units"]:
        for f in u.get("functions", []):
            for d in f.get("diagnostics", []):
                key = (u["library"], u["file"], f["function"], d.get("line"),
                       d.get("category"), d.get("description"))
                out[key] = d.get("origin")
    return out


def positions(data):
    """(library, file, function, line) -> list of (category, description)."""
    out = collections.defaultdict(list)
    for u in data["units"]:
        for f in u.get("functions", []):
            for d in f.get("diagnostics", []):
                out[(u["library"], u["file"], f["function"], d.get("line"))].append(
                    (d.get("category"), d.get("description"))
                )
    return out


def main() -> int:
    before = load(pathlib.Path(sys.argv[1]))
    after = load(pathlib.Path(sys.argv[2]))
    bl, al = leaves(before), leaves(after)
    bp, ap = positions(before), positions(after)

    removed = sorted(set(bl) - set(al))
    added = sorted(set(al) - set(bl))
    print(f"leaves removed: {len(removed)}   leaves added: {len(added)}   "
          f"net: {len(added) - len(removed):+d}")

    print()
    print("=== REMOVED, by class ===")
    by_callee = collections.Counter()
    other = []
    for k in removed:
        m = PAT.match(k[5] or "")
        if m:
            by_callee[m.group(1)] += 1
        else:
            other.append(k)
    declared_total = sum(v for n, v in by_callee.items() if n in DECLARED)
    print(f"  uncontracted-call leaves naming a DECLARED callee: {declared_total}")
    for n, v in sorted(by_callee.items(), key=lambda kv: -kv[1]):
        tag = "declared" if n in DECLARED else "CASCADE (not declared)"
        print(f"      {v:4d}  {n:40s} {tag}")
    print(f"  removed leaves that are NOT uncontracted-call rows: {len(other)}")
    for k in other:
        print(f"      {k[0]}/{k[1]} :: {k[2]} L{k[3]} [{k[4]}] {k[5]}")

    print()
    print("=== ADDED, split into SUBSTITUTION vs GENUINE ADDITION ===")
    subs, genuine = [], []
    for k in added:
        lib, file, fn, line, cat, desc = k
        had = bp.get((lib, file, fn, line), [])
        now = ap.get((lib, file, fn, line), [])
        # a substitution: this position carried a leaf before and carries the
        # same NUMBER of leaves now, so nothing was added at this position
        if had and len(had) == len(now):
            subs.append((k, had))
        else:
            genuine.append((k, had, now))
    print(f"  SAME-POSITION TEXT SUBSTITUTIONS (no leaf added here): {len(subs)}")
    for k, had in subs:
        print(f"    {k[0]}/{k[1]} :: {k[2]} L{k[3]}")
        for c, d in had:
            print(f"        was: [{c}] {d}")
        print(f"        now: [{k[4]}] {k[5]}")
    print(f"  GENUINE ADDITIONS (this position gained a leaf): {len(genuine)}")
    for k, had, now in genuine:
        print(f"    {k[0]}/{k[1]} :: {k[2]} L{k[3]} [{k[4]}] {k[5]}")
        print(f"        position had {len(had)} leaf/leaves before, {len(now)} now")

    print()
    print("=== withdrawal leaves in AFTER, by callee ===")
    w = collections.Counter()
    for k in al:
        m = WITHDRAWN.match(k[5] or "")
        if m:
            w[m.group(1)] += 1
    print(f"  {dict(w)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
