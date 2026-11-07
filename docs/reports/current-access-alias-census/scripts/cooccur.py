#!/usr/bin/env python3
"""Card CURRENT-ACCESS-ALIAS-CENSUS: what else blocks the functions a bundle touches.

usage: cooccur.py SHAPE_DIR OUT_JSON

For each bundle of rank.py (coarse tags) and each Unknown function row it
touches: the families of the items outside the candidate families (the next
blocker once the bundle and the other access/alias shapes are gone), counted
per function by the first of UNC-RUN, UNC-EXT, a fix-family, in that order.
"""
import collections
import gzip
import json
import pathlib
import sys

shape_dir, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
rows = [json.loads(line) for line in gzip.open(shape_dir / "functions.jsonl.gz", "rt")]
CAND = {"IND", "COND", "RET", "ALIAS"}


def coarse(tag):
    if not tag.startswith("expr:"):
        return tag
    _, construct, position, _roots = tag.split(":", 3)
    kind = "cond" if construct.startswith("cond") else construct
    return f"expr:{kind}:{'call-arg' if 'call-arg' in position else 'direct'}"


BUNDLES = {
    "K1": {"expr:statement:call-arg", "expr:return:call-arg", "expr:cond:call-arg"},
    "K2": {"none"},
    "K3": {"pointer-valued-store"},
    "K4": {"alias:formal-handed-uncontracted", "alias:formal-handed-contracted", "alias:formal-copied",
           "alias:formal-not-handed"},
    "K5": {"alias:derived-local-unguarded", "alias:guarded-other", "alias:borrowed-path-unguarded",
           "alias:contracted-result-unguarded", "alias:uncontracted-result-unguarded", "alias:non-pointer-name",
           "alias:statement-expression-unguarded", "alias:poisoned-by-refusal", "alias:address-taken",
           "alias:field-store-owner"},
    "K6": {"expr:statement:direct", "expr:return:direct", "expr:cond:direct"},
    "K7": {"union", "aggregate-value", "call", "indirect-call", "cast", "pointer-arithmetic", "static-local",
           "statement-expression", "pointer-step", "root-other", "function-designator", "address-of-base",
           "volatile-atomic", "pointer-field-step"},
}
result = {}
for name, tags in BUNDLES.items():
    counter = collections.Counter()
    touched = 0
    for r in rows:
        if r["status"] != "Unknown":
            continue
        mine = any(f in CAND and any(coarse(t) in tags for t in ts) for f, ts, _ in r["items"])
        if not mine:
            continue
        touched += 1
        others = {f for f, ts, _ in r["items"] if f not in CAND}
        if "UNC-RUN" in others:
            counter["UNC-RUN"] += 1
        elif "UNC-EXT" in others:
            counter["UNC-EXT"] += 1
        elif others:
            counter["other:" + sorted(others)[0]] += 1
        else:
            counter["no non-access blocker"] += 1
    result[name] = {"touched": touched, "next_blocker": dict(counter.most_common())}
out.write_text(json.dumps(result, indent=1) + "\n")
print(json.dumps(result, indent=1))
