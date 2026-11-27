#!/usr/bin/env python3
"""Spot-check one census shape against the corpus it came from.

Prints, for every leaf whose hardest statement has the requested shape, the
unit, function, INTERNAL preprocessed line (the leaf's identity, constraint
12a) and the PRESENTED origin coordinate the document carries beside it, plus
the access the census read. The presented coordinate is for a human to go and
look at the original source with; it is never what the census classified on —
that was the abandoned first card's error.

Usage: python3 spotcheck.py <run-dir> <shape-substring> [limit]
"""
import collections
import json
import pathlib
import sys

INDIRECT = "indirect memory access is outside the resource projection"
STEP = {"Arrow": "->", "Dot": ".", "Subscript": "[]", "Deref": "*"}

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from analyse import load_document, load_census, match_units, shape  # noqa: E402


def main():
    run = pathlib.Path(sys.argv[1])
    wanted = sys.argv[2]
    limit = int(sys.argv[3]) if len(sys.argv) > 3 else 40

    doc = json.load(open(run / "resource-diagnostics.json"))
    origins = {}
    sole_only = {}
    for unit in doc["units"]:
        ukey = (unit.get("library"), unit["file"])
        for function in unit["functions"]:
            families = {d["description"] for d in function["diagnostics"]}
            for d in function["diagnostics"]:
                if d["description"] == INDIRECT:
                    key = ukey + (function["function"], d["line"])
                    origins[key] = d.get("origin")
                    sole_only[key] = families == {INDIRECT}

    blockers, unit_leaves, leaf_functions = load_document(run / "resource-diagnostics.json")
    census, _ = load_census(run / "indirect-leaves.jsonl")
    assignment = match_units(census, unit_leaves)

    rows = []
    for ukey, keys in unit_leaves.items():
        records = census.get(assignment.get(ukey), {})
        for key in keys:
            got = records.get(key, [])
            if not got:
                continue
            hardest = max(got, key=shape)
            text = " | ".join(str(x) for x in shape(hardest))
            if wanted not in text:
                continue
            chain = max(hardest["chains"],
                        key=lambda c: (c["base_may_bind"], c["depth"])) \
                if hardest["chains"] else None
            rows.append((ukey, key, hardest, chain, text))

    print("shape filter: %r" % wanted)
    print("leaves matching: %d   (of which in sole-blocked functions: %d)"
          % (len(rows),
             sum(1 for u, k, _, _, _ in rows if sole_only.get(u + k))))
    print()
    print("%-26s %-34s %8s %-26s %s"
          % ("unit", "function", "ppline", "origin (presented)", "access"))
    for ukey, key, record, chain, text in rows[:limit]:
        access = "-"
        if chain:
            access = "%s%s%s" % (chain["base"] or "<call result>",
                                 "".join(STEP[s] for s in chain["steps"]),
                                 chain["member"] or "")
        print("%-26s %-34s %8d %-26s %s%s"
              % (str(ukey[1])[-26:], key[0][:34], key[1],
                 str(origins.get(ukey + key))[-26:], access,
                 "   [SOLE-BLOCKED]" if sole_only.get(ukey + key) else ""))

    kinds = collections.Counter(t for _, _, _, _, t in rows)
    print()
    print("exact shapes under this filter:")
    for name, count in kinds.most_common(10):
        print("   %5d  %s" % (count, name))


if __name__ == "__main__":
    main()
