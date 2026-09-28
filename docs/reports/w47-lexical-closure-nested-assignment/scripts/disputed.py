#!/usr/bin/env python3
"""Re-emit rule 10l's disputed set after rule 10o.

Usage: disputed.py PARITY_JSONL OUT_JSON

For every admitted typed row (P2 statement or P3 position the typed reader
admitted after the older routes refused) of a function the lexical
declaration reader reads, and every root of that row:

* `served-never-bound-while-ast-binds`: the chain says `root_never_bound:
  true` while the AST closure binds the root's spelling in that function
  (must be zero wherever the AST reader answers the function);
* `lexical-never-bound-while-ast-binds`: the (extended) lexical closure calls
  the root never bound while the AST binds it — what the join adds for the
  typed rows and what the lexical closure alone still serves to the non-typed
  consumers — by the AST's binding reason (declared names) or `file-scope`.

Also the census of the assignments the statement reading skipped.
"""
import collections
import json
import re
import sys

census = collections.Counter()
parenthesised = collections.Counter()
# Each analysis writes its root-declarations row before its typed rows (the
# stream is folded in unit order), so a typed row belongs to the last
# root-declarations row of its function: a function analysed several times
# (a header body, a second contract hold) is judged per analysis.
current = {}
rows = []
for line in open(sys.argv[1]):
    row = json.loads(line)
    if row.get("position") == "root-declarations":
        entry = {"readable": row.get("lexical_readable"),
                 "ast": set(row.get("ast_binds_spellings", [])),
                 "risky": set(row.get("lexical_risky", [])),
                 "reasons": {n["name"]: n["ast_binding"] for n in row.get("names", [])
                             if n.get("ast_binding")}}
        current[row["function"]] = entry
        nested = row.get("nested_census", {})
        census["unread_by_statement"] += nested.get("unread_by_statement", 0)
        census["nested_member_stores"] += nested.get("nested_member_stores", 0)
        census["analyses"] += 1
        for name in nested.get("parenthesised", []):
            parenthesised[name] += 1
    elif row.get("old_route") == "old_refusal" and row.get("new_admitted"):
        rows.append((row, current.get(row.get("function"))))

served = []
lexical = collections.Counter()
lexical_examples = collections.defaultdict(list)
for row, entry in rows:
    if not entry or not entry["readable"]:
        continue
    chains = row.get("new_chains") or ""
    for root, never in re.findall(r'root: "([^"]+)", dereferenced_root: \w+, root_never_bound: (\w+)', chains):
        if root not in entry["ast"]:
            continue
        if never == "true":
            served.append({"function": row["function"], "line": row["line"], "root": root})
        if root not in entry["risky"]:
            reason = entry["reasons"].get(root, "file-scope or not answered")
            klass = reason.split(":")[1] if reason.startswith("assignment:") else reason
            lexical[klass] += 1
            if len(lexical_examples[klass]) < 8:
                lexical_examples[klass].append(f'{row["function"]}:{root}')

out = {
    "admitted_typed_rows": len(rows),
    "served_never_bound_while_ast_binds": served,
    "lexical_never_bound_while_ast_binds_by_class": dict(sorted(lexical.items())),
    "examples": dict(lexical_examples),
    "nested_census": dict(census),
    "parenthesised_destinations": {"records": sum(parenthesised.values()), "distinct": len(parenthesised),
                                   "top": parenthesised.most_common(15)},
}
json.dump(out, open(sys.argv[2], "w"), indent=1)
print(json.dumps({k: v for k, v in out.items() if k != "examples"}, indent=1))
