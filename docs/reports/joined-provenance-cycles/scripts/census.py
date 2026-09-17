#!/usr/bin/env python3
"""Card JOINED-PROVENANCE-CYCLES: the population the acyclic pre-pass declines.

Ground truth comes from the analyzer's own CFG, emitted by the temporary
`QCORE_JPC_MEASURE` instrumentation in `legacy_joined_provenance::derive`, not
from a source-text classifier. Wave 33's reach log and the wave-34 brief were
both cut on a number that measured the wrong thing; this census therefore
reports three nested populations and says which one a design can serve.

Usage: python3 measure/joined-provenance-cycles/census.py
"""
import collections
import json
import pathlib
import sys

M = pathlib.Path(__file__).resolve().parent
MEASURE = pathlib.Path("/tmp/jpc/measure2.jsonl")


def instrumentation():
    """function name -> record. A function is analysed more than once (W4
    derivation, validation by re-analysis, certificate recheck); the CFG is the
    same each time, so collapse by name and check that it really is."""
    byname, disagreed = {}, set()
    for raw in MEASURE.read_text().splitlines():
        if not raw.strip():
            continue
        r = json.loads(raw)
        key = r["function"]
        if key in byname:
            if byname[key] != r:
                disagreed.add(key)
        byname[key] = r
    return byname, disagreed


def leaves(stage):
    doc = json.loads((M / "runs" / stage / "resource-diagnostics.json").read_text())
    per = collections.Counter()
    status = {}
    for u in doc["units"]:
        for f in u.get("functions") or []:
            n = sum(
                1 for d in (f.get("diagnostics") or []) if d["category"] == "unknown_alias"
            )
            per[f["function"]] += n
            status[f["function"]] = f["status"]
    return per, status


def main():
    if not MEASURE.exists():
        print(f"missing {MEASURE}", file=sys.stderr)
        return 2
    cfgs, disagreed = instrumentation()
    per, status = leaves("before")

    print(f"functions the instrumentation saw:        {len(cfgs)}")
    print(f"  of those, CFG differed between runs:    {len(disagreed)}")
    cyclic = {k: v for k, v in cfgs.items() if v["cyclic"]}
    print(f"  with a cyclic CFG:                      {len(cyclic)}")
    print(f"functions in the diagnostics document:    {len(per)}")
    print()

    # Population 1: what the pre-pass declines outright.
    p1 = {k: v for k, v in cyclic.items() if per.get(k, 0) > 0}
    print("== population 1: cyclic CFG AND >=1 unresolved-identity leaf ==")
    print(f"  functions: {len(p1)}")
    print(f"  leaves in them: {sum(per[k] for k in p1)}")

    # Population 2: those that actually acquire something. A function with no
    # acquisition site has nothing for the relation to be about, so no fixed
    # point over its CFG can produce a fact.
    p2 = {k: v for k, v in p1.items() if v["sites"] > 0}
    print("\n== population 2: ... and >=1 acquisition site in the CFG ==")
    print(f"  functions: {len(p2)}")
    print(f"  leaves in them: {sum(per[k] for k in p2)}")

    # Population 3: those with an acquisition that executes at most once per
    # call (its node lies on no cycle). A site inside a loop stands for many
    # dynamic objects, so a per-site identity fact would not be about one of
    # them; this is the sub-population a cycle-sound design can serve.
    p3 = {k: v for k, v in p2.items() if v["sites_off_cycle"] > 0}
    print("\n== population 3: ... and >=1 acquisition site lying on no cycle ==")
    print(f"  functions: {len(p3)}")
    print(f"  leaves in them: {sum(per[k] for k in p3)}")

    st = collections.Counter(status.get(k, "?") for k in p3)
    print(f"  their unit-projected status: {dict(st)}")

    print("\n== the 20 largest members of population 3 ==")
    for k in sorted(p3, key=lambda k: -per[k])[:20]:
        v = p3[k]
        print(
            f"  {per[k]:>4} leaves  sites={v['sites']:>3}"
            f" off-cycle={v['sites_off_cycle']:>3}"
            f"  names={v['conditional_names']:>3}  {k}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
