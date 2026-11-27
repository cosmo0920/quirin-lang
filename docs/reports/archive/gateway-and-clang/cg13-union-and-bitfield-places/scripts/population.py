#!/usr/bin/env python3
"""CG13-UNION-AND-BITFIELD-PLACES: re-derive CG00's M06/M07 population on a
new base without a new scanner.

usage: population.py DIAGNOSTICS OUT_JSON

CG00's leaf attribution (`../../cg00-clang-gap-baseline/artifacts/
attribution-leaves.jsonl.gz`, corpus `ae17152b...`) assigned every unsupported
leaf one entry and card. The corpus inputs are pinned, so a leaf of the
current base that has the same unit file, function, internal line and
description (backquoted names read as `X`, as CG00 recorded them) as a CG00 leaf is the same leaf, and keeps CG00's attribution. A
current leaf with no CG00 counterpart is `unattributed:new` (a blocker of its
own). A CG00 leaf with no current counterpart has moved (another card's route
now reads it, or the function changed status).

For every Unknown function of DIAGNOSTICS the blockers are the set of cards of
its unsupported leaves, as in CG00's `functions.py`: CG13 frees it **alone**
when that set is exactly {CG13}, with no leak or summary conflict; `strict`
also requires no `unknown_alias` leaf. Upper bounds, as CG00's.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
LEAVES = HERE.parents[1] / "cg00-clang-gap-baseline/artifacts/attribution-leaves.jsonl.gz"


def main():
    diagnostics = json.load(open(sys.argv[1]))
    out = sys.argv[2]
    with gzip.open(LEAVES, "rt") as handle:
        rows = [json.loads(line) for line in handle][1:]
    old = collections.defaultdict(list)
    for row in rows:
        old[(row["file"], row["function"], row["line"], row["description"])].append(row)

    status = {}
    alias = collections.Counter()
    other = collections.Counter()
    blockers = collections.defaultdict(set)
    current_m = collections.Counter()
    current_m_functions = collections.defaultdict(set)
    matched = set()
    new_leaves = 0
    for unit in diagnostics["units"]:
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            status[key] = function["status"]
            used = collections.Counter()
            for leaf in function["diagnostics"]:
                if leaf["category"] == "unknown_alias":
                    alias[key] += 1
                    continue
                if leaf["category"] != "unsupported":
                    other[key] += 1
                    continue
                lookup = (unit["file"], function["function"], leaf["line"],
                          re.sub(r"`[^`]*`", "`X`", leaf["description"]))
                candidates = old.get(lookup, [])
                if used[lookup] < len(candidates):
                    row = candidates[used[lookup]]
                    used[lookup] += 1
                    matched.add(id(row))
                    card = row["card"] if row["card"] != "unattributed" else row["entry"]
                    blockers[key].add(card)
                    if row["entry"] in ("M06", "M07"):
                        current_m[(row["entry"], row["reason"])] += 1
                        current_m_functions[row["entry"]].add(key)
                else:
                    new_leaves += 1
                    blockers[key].add("unattributed:new")
    moved_m = collections.Counter(
        (row["entry"], row["reason"])
        for row in rows
        if row["entry"] in ("M06", "M07") and id(row) not in matched
    )
    alone = {"with_alias": [], "strict": []}
    pair = collections.Counter()
    for key, state in sorted(status.items()):
        if state != "Unknown" or other[key]:
            continue
        cards = blockers.get(key, set())
        name = f"{key[0]}::{key[1]}"
        if cards == {"CG13"}:
            alone["with_alias"].append(name)
            if not alias[key]:
                alone["strict"].append(name)
        elif "CG13" in cards and len(cards) == 2:
            pair[next(iter(cards - {"CG13"}))] += 1
    cg00 = set()
    functions = json.load(open(HERE.parents[1] / "cg00-clang-gap-baseline/artifacts/attribution-functions.json"))
    cg00 = set(functions["freed_alone"]["with_alias"].get("CG13", []))
    result = {
        "schema": "qcore.cg13-population.v1",
        "status": dict(collections.Counter(status.values())),
        "current_leaves": {f"{e} {r}": n for (e, r), n in sorted(current_m.items())},
        "current_leaf_total": {e: sum(n for (x, _), n in current_m.items() if x == e) for e in ("M06", "M07")},
        "current_unknown_functions": {e: sum(1 for k in current_m_functions[e] if status[k] == "Unknown")
                                      for e in ("M06", "M07")},
        "moved_leaves": {f"{e} {r}": n for (e, r), n in sorted(moved_m.items())},
        "new_unattributed_leaves": new_leaves,
        "freed_alone": alone,
        "freed_alone_counts": {k: len(v) for k, v in alone.items()},
        "freed_with_one_other_card": dict(pair.most_common()),
        "cg00_alone_list_now": {
            "still_alone": sorted(cg00 & set(alone["with_alias"])),
            "already_safe": sorted(n for n in cg00 if status.get(tuple(n.split("::", 1))) == "Safe"),
            "other": sorted(n for n in cg00 - set(alone["with_alias"])
                            if status.get(tuple(n.split("::", 1))) != "Safe"),
        },
    }
    with open(out, "w") as handle:
        json.dump(result, handle, indent=1, sort_keys=True)
        handle.write("\n")
    print(json.dumps({k: result[k] for k in ("status", "current_leaf_total", "current_unknown_functions",
                                             "new_unattributed_leaves", "freed_alone_counts",
                                             "freed_with_one_other_card")}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
