#!/usr/bin/env python3
"""Card W37-CORPUS-CONTRACTS-4: leaf-level BEFORE/AFTER census.

Every moved leaf is printed individually, at the INTERNAL preprocessed
coordinate that is the leaf's identity (constraint 12a), with its presented
`origin` beside it. Nothing is summarised away into a count.

Usage: python3 -B measure/w36/census.py --before <dir> --after <dir>
"""
from __future__ import annotations

import argparse
import collections
import gzip
import json
import pathlib
import re

PAT = re.compile(r"^call to uncontracted function `([^`]+)`")
DECLARED = {
    "rd_kafka_topic_partition_list_sort",
    "mpack_write_cstr",
    "mpack_write_str",
    "mpack_write_i64",
    "mpack_write_double",
    "cmt_metric_get_timestamp",
    "cmt_metric_set_double",
    "cmt_mpack_consume_uint_tag",
    "cfl_time_now",
    "LZ4_resetStream",
}


def load(where: pathlib.Path) -> dict:
    gz = where / "diagnostics.json.gz"
    if gz.exists():
        return json.loads(gzip.decompress(gz.read_bytes()))
    return json.loads((where / "resource-diagnostics.json").read_bytes())


def leaves(data: dict) -> dict:
    """(library, file, function, line, category, description) -> origin."""
    out = {}
    for u in data["units"]:
        for f in u.get("functions", []):
            for d in f.get("diagnostics", []):
                key = (
                    u["library"],
                    u["file"],
                    f["function"],
                    d.get("line"),
                    d.get("category"),
                    d.get("description"),
                )
                out[key] = d.get("origin")
    return out


def statuses(data: dict):
    units = {(u["library"], u["file"]): u.get("status") for u in data["units"]}
    funcs = {
        (u["library"], u["file"], f["function"]): f.get("status")
        for u in data["units"]
        for f in u.get("functions", [])
    }
    return units, funcs


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--before", required=True)
    ap.add_argument("--after", required=True)
    args = ap.parse_args()
    before = load(pathlib.Path(args.before))
    after = load(pathlib.Path(args.after))

    bl, al = leaves(before), leaves(after)
    bu, bf = statuses(before)
    au, af = statuses(after)

    print("=== corpus diagnostic counts ===")
    bc: collections.Counter = collections.Counter()
    ac: collections.Counter = collections.Counter()
    for u in before["units"]:
        bc.update(u.get("diagnostic_counts", {}))
    for u in after["units"]:
        ac.update(u.get("diagnostic_counts", {}))
    for k in sorted(set(bc) | set(ac)):
        mark = "" if bc[k] == ac[k] else "   <-- MOVED"
        print(f"  {k:16s} {bc[k]:6d} -> {ac[k]:6d}{mark}")

    print()
    print("=== unit status moves ===")
    moved_u = [k for k in bu if bu[k] != au.get(k)]
    print(f"  {len(moved_u)} unit(s) moved")
    for k in sorted(moved_u):
        print(f"    {k}: {bu[k]} -> {au.get(k)}")

    print()
    print("=== function status moves ===")
    moved_f = [k for k in bf if bf[k] != af.get(k)]
    print(f"  {len(moved_f)} function(s) moved")
    for k in sorted(moved_f):
        print(f"    {k[0]}/{k[1]} :: {k[2]}: {bf[k]} -> {af.get(k)}")

    removed = sorted(set(bl) - set(al))
    added = sorted(set(al) - set(bl))
    print()
    print(f"=== leaves REMOVED: {len(removed)} ===")
    cat = collections.Counter(k[4] for k in removed)
    print(f"  by category: {dict(sorted(cat.items()))}")
    named = collections.Counter()
    for k in removed:
        m = PAT.match(k[5] or "")
        if m:
            named[m.group(1)] += 1
    print(f"  uncontracted-call leaves by callee: {dict(named.most_common())}")
    print(f"  of those, naming a callee this card declared: "
          f"{sum(v for n, v in named.items() if n in DECLARED)}")
    print(f"  removed leaves NOT naming a declared callee: "
          f"{len(removed) - sum(v for n, v in named.items() if n in DECLARED)}")

    print()
    print(f"=== leaves ADDED: {len(added)} ===")
    cat = collections.Counter(k[4] for k in added)
    print(f"  by category: {dict(sorted(cat.items()))}")
    for k in added:
        print(f"    {k[0]}/{k[1]} :: {k[2]} L{k[3]} origin={al[k]} [{k[4]}] {k[5]}")

    # The three-way split the card must publish, over the predicted functions.
    print()
    print("=== three-way split over the 11 predicted functions ===")
    predicted = []
    for u in before["units"]:
        for f in u.get("functions", []):
            lv = f.get("diagnostics", [])
            if not lv:
                continue
            names, only = set(), True
            for d in lv:
                m = PAT.match(d.get("description") or "")
                if m:
                    names.add(m.group(1))
                else:
                    only = False
                    break
            if only and len(names) == 1 and next(iter(names)) in DECLARED:
                predicted.append((u["library"], u["file"], f["function"], next(iter(names))))
    leaf_free, exchanged, unmoved = [], [], []
    for lib, file, fn, callee in sorted(predicted):
        rows = []
        for u in after["units"]:
            if u["library"] == lib and u["file"] == file:
                for f in u.get("functions", []):
                    if f["function"] == fn:
                        rows = f.get("diagnostics", [])
        if not rows:
            leaf_free.append((lib, file, fn, callee))
        elif any(PAT.match(d.get("description") or "") and
                 PAT.match(d["description"]).group(1) == callee for d in rows):
            unmoved.append((lib, file, fn, callee, rows))
        else:
            exchanged.append((lib, file, fn, callee, rows))
    print(f"  LEAF-FREE (every leaf gone):        {len(leaf_free)}")
    for lib, file, fn, callee in leaf_free:
        print(f"    {lib}/{file} :: {fn}  ({callee})  status={af.get((lib, file, fn))}")
    print(f"  EXCHANGED for another refusal:      {len(exchanged)}")
    for lib, file, fn, callee, rows in exchanged:
        print(f"    {lib}/{file} :: {fn}  ({callee})  status={af.get((lib, file, fn))}")
        for d in rows:
            print(f"        L{d.get('line')} origin={d.get('origin')} "
                  f"[{d.get('category')}] {d.get('description')}")
    print(f"  UNMOVED (the leaf is still there):  {len(unmoved)}")
    for lib, file, fn, callee, rows in unmoved:
        print(f"    {lib}/{file} :: {fn}  ({callee})  status={af.get((lib, file, fn))}")
        for d in rows:
            print(f"        L{d.get('line')} origin={d.get('origin')} "
                  f"[{d.get('category')}] {d.get('description')}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
