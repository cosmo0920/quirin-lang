#!/usr/bin/env python3
"""Leaves / Unknown functions / sole-blocked Unknown functions of the STATUS
refusal families, for one diagnostics document.

Usage: family_counts.py DIAGNOSTICS_JSON[.gz]

A family's leaves are counted on Unknown functions; a function is sole-blocked
when every one of its leaves belongs to the family. Families: uncontracted
call (`call to uncontracted function`), indirect access (`indirect memory
access is outside the resource projection`), return expression (`unmodeled
call, aggregate or indirect access in return expression`).
"""
import gzip
import json
import pathlib
import sys

FAMILIES = {
    "uncontracted_call": lambda d: d.startswith("call to uncontracted function"),
    "indirect_access": lambda d: d == "indirect memory access is outside the resource projection",
    "return_expression": lambda d: d == "unmodeled call, aggregate or indirect access in return expression",
}

raw = pathlib.Path(sys.argv[1]).read_bytes()
if sys.argv[1].endswith(".gz"):
    raw = gzip.decompress(raw)
document = json.loads(raw)
out = {}
for name, member in FAMILIES.items():
    leaves = functions = sole = 0
    for unit in document["units"]:
        for function in unit["functions"]:
            if function["status"] != "Unknown":
                continue
            hits = [leaf for leaf in function["diagnostics"] if member(leaf["description"])]
            if hits:
                leaves += len(hits)
                functions += 1
                if len(hits) == len(function["diagnostics"]):
                    sole += 1
    out[name] = {"leaves": leaves, "functions": functions, "sole": sole}
print(json.dumps(out, indent=1))
