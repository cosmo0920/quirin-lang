#!/usr/bin/env python3
"""reasons.py RECENSUS.json RUN_DIR OUT.json -- K7: for every K7 leaf of a row
the bundle frees alone, the typed-path parity stream's rows at the leaf's
(function, line) (QCORE_TYPED_PATH_PARITY, the analyzer's own consultations:
position, admitted, refusal), and whether the unit-scoped run reproduces the
user's row (status and leaves). The stream keys rows by function and line
only; a reading aid, not a verdict input."""
import collections
import json
import pathlib
import sys

recensus = json.load(open(sys.argv[1]))
run = pathlib.Path(sys.argv[2])
parity = collections.defaultdict(list)
with open(run / "parity.jsonl") as handle:
    for line in handle:
        row = json.loads(line)
        if row.get("function") and row.get("line"):
            parity[(row["function"], row["line"])].append(row)
report = json.load(open(run / "resource-diagnostics.json"))
rows = {}
for unit in report["units"]:
    for function in unit["functions"]:
        rows[(unit["file"], function["function"])] = function

out = []
for r in recensus["freed_rows"] + recensus["loose_rows"]:
    mine = rows.get((r["file"], r["function"]))
    entry = {"file": r["file"], "function": r["function"], "items": r["items"],
             "reproduced_status": mine["status"] if mine else None,
             "reproduced_leaves": sorted(f"{d['category']}:{d['description']}@{d['line']}"
                                         for d in mine["diagnostics"]) if mine else None,
             "leaves": []}
    for leaf in r["k7"]:
        consultations = []
        for p in parity.get((r["function"], leaf["line"]), []):
            consultations.append({k: p.get(k) for k in ("position", "new_admitted", "new_refusal", "old_admitted",
                                                        "old_refusal", "access", "path", "text") if k in p})
        entry["leaves"].append({"line": leaf["line"], "origin": leaf["origin"], "tags": leaf["tags"],
                                "consultations": consultations})
    out.append(entry)
pathlib.Path(sys.argv[3]).write_text(json.dumps(out, indent=1) + "\n")
for e in out:
    print("#", e["function"], e["reproduced_status"], e["items"])
    for leaf in e["leaves"]:
        print("  ", leaf["origin"], leaf["tags"])
        seen = set()
        for c in leaf["consultations"]:
            text = f"{c.get('position')} admitted={c.get('new_admitted')} {str(c.get('new_refusal'))[:160]}"
            if text not in seen:
                seen.add(text)
                print("      ", text)
