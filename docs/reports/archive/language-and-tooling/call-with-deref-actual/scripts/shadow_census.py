#!/usr/bin/env python3
"""shadow_census.py REPORT CHECKOUT -- CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION:
every leaf "local declaration may shadow a formal resource identity" of one
opt-in `current` report whose source line opens `IDENT ( *` (the rule-89
prefix; `if`/`while`/`return`/`sizeof`/... excluded), with its row's status
and whether the shadow leaves are its only leaves. A line test only: the
rule itself asks Clang, so this bounds the candidates, it decides nothing."""
import json
import os
import re
import sys

SHADOW = "local declaration may shadow a formal resource identity"
PREFIX = re.compile(r"^\s*([A-Za-z_][A-Za-z_0-9]*)\s*\(\s*\*")
KEYWORDS = {"if", "while", "for", "switch", "return", "sizeof", "do", "else", "case"}


def main() -> int:
    data = json.load(open(sys.argv[1]))
    checkout = sys.argv[2]
    cache = {}
    rows = 0
    for unit in data["units"]:
        path = unit["file"]
        full = path if os.path.isabs(path) else os.path.join(checkout, path)
        for function in unit.get("functions", []):
            leaves = function.get("diagnostics", [])
            shadow = [d for d in leaves if d["description"] == SHADOW]
            if not shadow:
                continue
            if full not in cache:
                try:
                    cache[full] = open(full, errors="replace").read().split("\n")
                except OSError:
                    cache[full] = None
            lines = cache[full]
            hits = []
            for leaf in shadow:
                line = leaf.get("line")
                text = lines[line - 1] if lines and line and line <= len(lines) else ""
                match = PREFIX.match(text)
                if match and match.group(1) not in KEYWORDS:
                    hits.append(f"{line}: {text.strip()}")
            if hits:
                rows += 1
                print(json.dumps({
                    "file": path, "function": function["function"],
                    "status": function["status"], "leaves": len(leaves),
                    "shadow_leaves": len(shadow), "prefix_hits": hits,
                }, sort_keys=True))
    print(json.dumps({"rows_with_prefix_shadow_leaves": rows}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
