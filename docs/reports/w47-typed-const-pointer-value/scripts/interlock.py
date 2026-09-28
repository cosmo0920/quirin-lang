#!/usr/bin/env python3
"""The common brief's soundness interlock: no function this card moves to
Safe may rest on a root the nested-assignment class disputes.

Usage: interlock.py AFTER_PARITY COMPARISON_JSON OUT_JSON

For every function the comparison moves to Safe, read its AFTER
`root-declarations` row(s) and the roots of its admitted typed rows. A name
is *disputed* when the AST closure binds it and the lexical closure calls it
never bound (rule 10l's `ast-binds-lexical-never-bound` class); the class
W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT tracks is the subset bound by
`assignment:call:nested`. The function passes when no admitted-row root is
disputed.
"""
import collections
import json
import re
import sys

parity, comparison, out = sys.argv[1:4]
moved = {e["function"] for e in json.load(open(comparison))["changed"]
         if e["status"][1] == "Safe" and e["status"][0] != "Safe"}
declarations = collections.defaultdict(list)
roots = collections.defaultdict(set)
for line in open(parity):
    row = json.loads(line)
    function = row.get("function")
    if function not in moved:
        continue
    if row.get("position") == "root-declarations":
        declarations[function].append(row)
        continue
    if not row.get("new_admitted"):
        continue
    text = (row.get("new_scalar") or "") + (row.get("new_chains") or "")
    roots[function].update(re.findall(r'root: "([^"]+)"', text))
    for bases in re.findall(r'bases: \[([^\]]*)\]', text):
        roots[function].update(re.findall(r'"([^"]+)"', bases))
result = []
for function in sorted(moved):
    disputed, nested = set(), set()
    for row in declarations[function]:
        for name in row.get("names", []):
            if name.get("ast_never_bound") is False and name.get("lexical_closure_never_bound") is True:
                disputed.add(name["name"])
                if (name.get("ast_binding") or "").startswith("assignment:call:nested"):
                    nested.add(name["name"])
    exposed = sorted(roots[function] & disputed)
    result.append({"function": function, "admitted_roots": sorted(roots[function]),
                   "disputed_names": sorted(disputed), "nested_call_names": sorted(nested),
                   "exposed": exposed, "passes": not exposed,
                   "root_declaration_rows": len(declarations[function])})
json.dump(result, open(out, "w"), indent=1)
for entry in result:
    print(("PASS " if entry["passes"] else "FAIL ") + entry["function"], entry["admitted_roots"],
          "disputed:", entry["disputed_names"])
print("moved to Safe:", len(result), "failing the interlock:", sum(not e["passes"] for e in result))
