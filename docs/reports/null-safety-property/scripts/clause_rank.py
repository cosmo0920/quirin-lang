#!/usr/bin/env python3
"""clause_rank.py ROWS.jsonl SOURCE_ROOT [OUT.json] -- NULL-SAFETY-PROPERTY:
rank the functions whose own unit inferred `quant null-deref #B;` (a formal
dereferenced on every terminating path before any test) by how many files of
the corpus source tree other than their defining file name them in a call
(`NAME (`, a textual count), keeping definitions without `static` (external
linkage). The top of the ranking is where a declared clause moves an
obligation to callers the unit's own inference cannot see (brief task 4)."""
import collections
import json
import os
import re
import sys

rows = [json.loads(line) for line in open(sys.argv[1]) if line.strip()]
root = sys.argv[2]
inferred = collections.defaultdict(set)
units = collections.defaultdict(set)
for row in rows:
    null = row.get("null") or {}
    if null.get("inferred_preconditions"):
        inferred[row["function"]].update(null["inferred_preconditions"])
        units[row["function"]].add(row["library"] + "/" + row["file"])

call = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")
# A definition head: a line starting a declaration, the name, a parameter
# list, and an opening brace before the next semicolon.
definition = re.compile(r"^([A-Za-z_][^;{}()]*?)\b([A-Za-z_][A-Za-z0-9_]*)\s*\(([^;{}]*)\)\s*\{", re.M)
calls = collections.defaultdict(collections.Counter)
defined = collections.defaultdict(list)
for base in ("lib", "src", "plugins", "include"):
    for directory, _, names in os.walk(os.path.join(root, base)):
        for name in names:
            if not name.endswith((".c", ".h")):
                continue
            path = os.path.join(directory, name)
            try:
                text = open(path, errors="replace").read()
            except OSError:
                continue
            relative = os.path.relpath(path, root)
            for match in call.finditer(text):
                if match.group(1) in inferred:
                    calls[match.group(1)][relative] += 1
            for match in definition.finditer(text):
                if match.group(2) in inferred:
                    defined[match.group(2)].append((relative, "static" in match.group(1).split()))

ranked = []
for name in inferred:
    definitions = defined.get(name, [])
    if not definitions or any(static for _, static in definitions):
        continue
    homes = {path for path, _ in definitions}
    others = {path: n for path, n in calls[name].items() if path not in homes}
    if not others:
        continue
    ranked.append({
        "function": name,
        "files_other_than_definitions": len(others),
        "call_sites_other_than_definitions": sum(others.values()),
        "definitions": sorted(homes),
        "inferred": sorted(inferred[name]),
        "units": sorted(units[name])[:3],
    })
ranked.sort(key=lambda r: (-r["files_other_than_definitions"], -r["call_sites_other_than_definitions"], r["function"]))
out = json.dumps(ranked[:80], indent=1)
if len(sys.argv) > 3:
    open(sys.argv[3], "w").write(out + "\n")
print(out[:8000])
