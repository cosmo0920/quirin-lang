#!/usr/bin/env python3
"""keyword_callees.py DIAGNOSTICS... -- size CG22-INLINE-FNPTR-LOCAL-DECLARATION:
leaves `call to uncontracted function `KEYWORD`` (a declaration such as
`void (*f)(void *) = g;` read lexically as a call of its type keyword), the
functions holding them, and those where they are every leaf."""
import collections
import json
import re
import sys

KEYWORD = re.compile(
    r"call to uncontracted function `(void|int|char|long|short|unsigned|signed|float|double|_Bool|const|struct|union|enum)`"
)

for path in sys.argv[1:]:
    data = json.load(open(path, encoding="utf-8"))
    leaves = collections.Counter()
    functions = set()
    sole = []
    for unit in data["units"]:
        for function in unit["functions"]:
            diagnostics = function.get("diagnostics", [])
            hits = [d for d in diagnostics if KEYWORD.match(d.get("description") or "")]
            if not hits:
                continue
            functions.add((unit["file"], function["function"]))
            for hit in hits:
                leaves[hit["description"]] += 1
            if len(hits) == len(diagnostics):
                sole.append((unit["file"], function["function"], function["status"]))
    print(json.dumps({"report": path, "leaves": sum(leaves.values()), "functions": len(functions),
                      "by_keyword": dict(leaves), "sole_blocker": sole}, indent=1))
