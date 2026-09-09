#!/usr/bin/env python3
"""Census of flb_hash_table_add call sites: the fifth actual's spelling."""
import os
import re
import sys

ROOT = sys.argv[1] if len(sys.argv) > 1 else "."
NAME = sys.argv[2] if len(sys.argv) > 2 else "flb_hash_table_add"

rows = []
for base, dirs, files in os.walk(ROOT):
    dirs[:] = [d for d in dirs if d not in (".git", "build", "tests")]
    for name in files:
        if not name.endswith((".c", ".h")):
            continue
        path = os.path.join(base, name)
        try:
            text = open(path, encoding="utf-8", errors="replace").read()
        except OSError:
            continue
        for m in re.finditer(re.escape(NAME) + r"\s*\(", text):
            start = m.end()
            # a definition/declaration, not a call
            depth = 1
            i = start
            args = []
            current = []
            while i < len(text) and depth > 0:
                c = text[i]
                if c in "([":
                    depth += 1
                elif c in ")]":
                    depth -= 1
                    if depth == 0:
                        args.append("".join(current))
                        break
                elif c == "," and depth == 1:
                    args.append("".join(current))
                    current = []
                    i += 1
                    continue
                current.append(c)
                i += 1
            line = text[: m.start()].count("\n") + 1
            joined = [" ".join(a.split()) for a in args]
            if len(joined) != 5:
                rows.append((path, line, "ARITY=%d" % len(joined), joined))
                continue
            rows.append((path, line, joined[4], joined))

kinds = {}
for path, line, last, args in rows:
    rel = os.path.relpath(path, ROOT)
    if last.startswith("ARITY"):
        kind = last
    elif re.fullmatch(r"0", last):
        kind = "literal 0 (exact)"
    elif re.fullmatch(r"\(\s*0\s*\)", last):
        kind = "parenthesised 0"
    elif re.fullmatch(r"0[uUlL]+", last):
        kind = "suffixed 0"
    elif re.fullmatch(r"-?\d+", last):
        kind = "other decimal literal"
    elif re.fullmatch(r"0[xX][0-9a-fA-F]+", last):
        kind = "hex literal"
    else:
        kind = "non-literal expression"
    kinds.setdefault(kind, []).append((rel, line, last))

for kind in sorted(kinds):
    print("== %s: %d" % (kind, len(kinds[kind])))
    for rel, line, last in sorted(kinds[kind]):
        print("   %s:%d  %s" % (rel, line, last))
print("TOTAL %d" % len(rows))
