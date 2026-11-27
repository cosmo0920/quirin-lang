#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card PRIORITY0-ATTRIBUTION: every new Safe of the chain record (a function
row Safe at tip and not at prev): its tip row's status, diagnostics,
contract dependencies and hypothesis context, read from the tip side's
resource-diagnostics.json (read-only).

    python3 new_safe_evidence.py CHAIN.json OUT.json"""
import json, sys, collections
M = "/media/Data/GitHub/quirin-lang/.claude/worktrees/priority0/private/measure/priority0-20261125T173228Z"
chain = json.load(open(sys.argv[1]))
out_path = sys.argv[2]
SUB = {"corpus": "corpus/corpus", "twelve-home": "twelve-home/corpus", "system": "system", "slice": "slice"}


def index(route):
    d = json.load(open(f"{M}/sides/tip/{SUB[route]}/resource-diagnostics.json"))
    idx = {}
    for unit in d["units"]:
        lib = unit.get("library") or unit.get("profile") or ""
        for f in unit.get("functions", []):
            idx[(unit.get("file") or unit.get("path"), f["function"])] = (unit, f)
    return d, idx


rows = []
for route, r in chain["segments"][0]["routes"].items():
    want = [x for x in r["rows"] if x["kind"] == "function" and x["verdicts"][-1] == "Safe" and x["verdicts"][0] != "Safe"]
    if not want:
        continue
    d, idx = index(route)
    keys = list(idx)[:1]
    for x in want:
        lib, file, unit, prop, lib2, fn = x["row"].split("::")
        fn = fn.split("#")[0]
        cands = [v for (f, g), v in idx.items() if g == fn and f and f.endswith(file)]
        unitno = int(unit.split("#")[1])
        if len(cands) > 1:
            cands = [c for c in cands if str(c[0].get("unit_index", c[0].get("index", ""))) == str(unitno)] or cands
        if not cands:
            rows.append({"route": route, "row": x["row"], "found": False})
            continue
        u, f = cands[min(unitno, len(cands) - 1)] if len(cands) > 1 else cands[0]
        cd = f.get("contract_dependencies", {})
        rows.append({"route": route, "row": x["row"], "found": True, "status": f["status"],
                     "diagnostics": len(f.get("diagnostics", [])),
                     "declarations": cd.get("declarations", []),
                     "hypothesis_dependencies": cd.get("hypothesis_dependencies", {}),
                     "summaries": cd.get("summaries", []), "null_premises": cd.get("null_premises", []),
                     "withdrawn": cd.get("withdrawn", []),
                     "assumptions": f.get("hypothesis_context", {}).get("assumptions"),
                     "other_keys": sorted(set(f) - {"function", "status", "diagnostics", "contract_dependencies", "hypothesis_context"})})
json.dump(rows, open(out_path, "w"), indent=1)
c = collections.Counter()
for x in rows:
    if not x["found"]:
        c["not found"] += 1
        continue
    c[(x["route"], x["status"], "diag=0" if x["diagnostics"] == 0 else "diag>0",
       "decl" if x["declarations"] else "nodecl", "hyp" if x["hypothesis_dependencies"] else "nohyp",
       "null" if x["null_premises"] else "nonull")] += 1
for k, v in sorted(c.items(), key=str):
    print(k, v)
print("units keys sample:", list(json.load(open(f"{M}/sides/tip/slice/resource-diagnostics.json"))["units"][0])[:12])
