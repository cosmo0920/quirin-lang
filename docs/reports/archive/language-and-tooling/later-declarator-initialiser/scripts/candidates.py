#!/usr/bin/env python3
"""candidates.py REPORT CHECKOUT -- LATER-DECLARATOR-INITIALISER-UNREAD's
prediction census over a BASE opt-in `current` report: every Unknown row
carrying the indirect-access leaf whose function body, read from the unit's
source file in CHECKOUT, holds a declaration of several init-declarators with
a later pointer declarator and an initialiser (`T *a = X, *b = Y;`, a rough
regex over the unpreprocessed source, comments not stripped). For each it
prints the row's leaf count, its access leaves, the number of such
declarations, and whether every leaf of the row is an access leaf (`alone`)."""
import json
import os
import re
import sys

GATE = "indirect memory access is outside the resource projection"
# specifiers, a first declarator with `=`, then a later `, *name =` before `;`
DECL = re.compile(
    r"^\s*(?:const\s+|struct\s+|unsigned\s+|signed\s+)*[A-Za-z_]\w*\s+\**\s*[A-Za-z_]\w*\s*"
    r"(?:=[^;{}]*)?,\s*\*+\s*[A-Za-z_]\w*\s*=[^;{}]*;",
    re.M,
)


def body(source, name):
    match = re.search(r"\b" + re.escape(name) + r"\s*\([^;{]*\)\s*\{", source)
    if not match:
        return None
    depth, at = 0, match.end() - 1
    for index in range(at, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[at:index + 1]
    return None


def main() -> int:
    data = json.load(open(sys.argv[1]))
    checkout = sys.argv[2]
    cache = {}
    for unit in data["units"]:
        path = unit["file"]
        if not os.path.isabs(path):
            path = os.path.join(checkout, path)
        if path not in cache:
            try:
                cache[path] = open(path, errors="replace").read()
            except OSError:
                cache[path] = None
        source = cache[path]
        if source is None:
            continue
        for function in unit["functions"]:
            if function["status"] != "Unknown":
                continue
            leaves = [d["description"] for d in function.get("diagnostics", [])]
            access = sum(1 for d in leaves if d.startswith(GATE))
            if not access:
                continue
            text = body(source, function["function"])
            if not text:
                continue
            found = DECL.findall(text)
            if not found:
                continue
            print(json.dumps({
                "file": unit["file"], "function": function["function"],
                "leaves": len(leaves), "access": access, "declarations": len(found),
                "alone": access == len(leaves),
            }, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
