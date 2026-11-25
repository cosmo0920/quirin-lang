#!/usr/bin/env python3
"""fate.py CENSUS_ARTIFACTS R1124_PICKLE MONKEY_OFFSETS OUT_JSON -- K2: what the
11-24 report holds at the (file, function, line) of every 11-07 `none` leaf group:
the same family's leaf (kept), AG3's named condition text (exchanged), another
diagnostic, or nothing (gone). A row that is Safe at 11-24 is counted apart."""
import collections
import gzip
import json
import pickle
import sys

art, pkl, offs, out = sys.argv[1:5]
OFF = {k: v for k, v in json.load(open(offs)).items() if k.startswith("lib/monkey/")}
units = pickle.load(open(pkl, "rb"))
at = collections.defaultdict(list)
status = {}
for d in units:
    for f in d["functions"]:
        status.setdefault((d["file"], f["function"]), set()).add(f["status"])
        for cat, desc, line, origin in f["diagnostics"]:
            at[(d["file"], f["function"], line + 0)].append((cat, desc))
fate = collections.Counter()
by_family = collections.defaultdict(collections.Counter)
with gzip.open(f"{art}/shape-leaves.jsonl.gz", "rt") as h:
    h.readline()
    for line in h:
        r = json.loads(line)
        if r["tags"] != ["none"]:
            continue
        key = (r["file"], r["function"], r["line"] + OFF.get(r["file"], 0))
        here = at.get(key, [])
        st = status.get((r["file"], r["function"]), set())
        if not st:
            kind = "function absent"
        elif st == {"Safe"}:
            kind = "row Safe"
        elif any(desc.startswith("unmodeled call, aggregate or indirect access in")
                 or desc == "indirect memory access is outside the resource projection" for _, desc in here):
            kind = "kept (same refusal text)"
        elif any(desc.startswith("call to uncontracted function") and " in " in desc for _, desc in here):
            kind = "exchanged for AG3's named callee text (rule 63 (2))"
        elif here:
            kind = "exchanged for another diagnostic: " + sorted({c for c, _ in here})[0]
        else:
            kind = "gone (no diagnostic at the line)"
        fate[kind] += r["leaves"]
        by_family[r["family"]][kind] += r["leaves"]
result = {"leaves": dict(fate), "by_family": {k: dict(v) for k, v in by_family.items()}}
json.dump(result, open(out, "w"), indent=1)
print(json.dumps(result, indent=1))
