#!/usr/bin/env python3
"""rows.py PREMISES.jsonl.gz PPDIR R3SCRIPTS OUT.json -- the embedded-list sole rows."""
import collections
import gzip
import json
import os
import sys

sys.path.insert(0, sys.argv[3])
from refine import shape, IND  # noqa: E402
from leaves import statement  # noqa: E402

rows = [json.loads(line) for line in gzip.open(sys.argv[1], "rt")]
pp = sys.argv[2]
by = collections.defaultdict(list)
for r in rows:
    if set(r["families"]) <= {IND} and r["status"] == "Unknown":
        by[(r["file"], r["function"])].append(r)
cache = {}
out = []
for key, leaves in sorted(by.items()):
    p = os.path.join(pp, f"unit-{leaves[0]['unit']}.c")
    if p not in cache:
        cache[p] = open(p, errors="replace").read().split("\n")
    lines = cache[p]
    shapes = set()
    det = []
    for lf in leaves:
        t = statement(lines, lf["line"] - 1)[0]
        n, _ = shape(lf, t)
        shapes.add(n)
        det.append(
            {
                "shape": n,
                "origin": lf["origin"],
                "origin_text": lf["origin_text"],
                "statement": t[:240],
                "premise": lf["premise"],
                "address_paths": [
                    {"callee": pa.get("callee"), "arg": pa.get("arg"), "root": pa["root"], "root_kind": pa["root_kind"],
                     "root_type": pa["root_type"], "steps": [(s["op"], s["member"], s["type"]) for s in pa["steps"]]}
                    for pa in lf["paths"]
                ],
            }
        )
    if shapes == {"embedded-list-address"}:
        out.append({"unit": leaves[0]["unit"], "file": key[0], "function": key[1], "leaves": det})
print(len(out))
json.dump(out, open(sys.argv[4], "w"), indent=1)
for o in out:
    print("##", o["file"], o["function"])
    for d in o["leaves"]:
        print("   ", d["origin"], "|", d["origin_text"][:150], "|", [(p["callee"], p["arg"], p["root"], p["root_kind"], [s[1] for s in p["steps"]]) for p in d["address_paths"]])
