#!/usr/bin/env python3
"""gains.py COMPARE.json AFTER.jsonl [OUT.json] -- FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE:
for every row that moved in COMPARE.json (compare.py's output), the AFTER
row's null verdict and scope, its check outcomes, every proof naming a
declaration (`precondition:declared:…`, `result:stable:…`, `moved:…`), its
preconditions, assumptions and contract dependencies, and its custody and
combined verdicts: the material a hand reading of each gain or correction
lists its clauses from."""
import collections
import json
import sys

compare = json.load(open(sys.argv[1]))
rows = {}
seen = collections.Counter()
for line in open(sys.argv[2]):
    row = json.loads(line)
    base = (row["library"], row["file"], row["function"])
    rows[base + (seen[base],)] = row
    seen[base] += 1

out = []
for move in compare["moving"]:
    row = rows[(move["library"], move["file"], move["function"], move["configuration"])]
    null = row["null"]
    proofs = collections.Counter()
    for check in null.get("checks") or []:
        for proof in check.get("proofs") or []:
            if proof.startswith(("precondition:", "result:stable:", "result:member:", "inferred:")) or "nonnull_member" in proof:
                proofs[proof] += 1
    out.append({
        "function": move["function"], "configuration": move["configuration"], "move": move["move"],
        "null_verdict": null.get("verdict"), "null_scope": null.get("scope"),
        "outcomes": dict(collections.Counter(c["outcome"] for c in null.get("checks") or [])),
        "candidates": [c["subject"] for c in null.get("checks") or [] if c["outcome"] == "candidate"],
        "declaration_proofs": dict(proofs),
        "fields": {k: null[k] for k in null if k not in ("checks", "verdict", "scope")},
        "custody": row.get("custody"), "combined": row.get("combined"),
    })
text = json.dumps(out, indent=1, ensure_ascii=False)
if len(sys.argv) > 3:
    open(sys.argv[3], "w").write(text + "\n")
print(text)
