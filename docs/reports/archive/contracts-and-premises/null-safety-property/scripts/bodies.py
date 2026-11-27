#!/usr/bin/env python3
"""bodies.py PATH:FUNCTION... -- NULL-SAFETY-PROPERTY: print each named
function's definition at both audited revisions (66910c10, the corpus tree;
046accf6, the prepared system tree, both read only), and whether the two
texts are byte-identical, for the review the brief asks before a `quant
null-deref` clause is attached."""
import hashlib
import re
import sys

OLD = "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"
NEW = "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source"


def body(root, path, name):
    try:
        text = open(f"{root}/{path}", errors="replace").read()
    except OSError as error:
        return f"<{error}>"
    for match in re.finditer(r"^[^\s#/][^;{}]*\b%s\s*\([^;{}]*\)\s*\{" % re.escape(name), text, re.M):
        start = match.start()
        depth = 0
        for index in range(match.end() - 1, len(text)):
            if text[index] == "{":
                depth += 1
            elif text[index] == "}":
                depth -= 1
                if depth == 0:
                    return text[start:index + 1]
    return "<no definition>"


limit = 60
for argument in sys.argv[1:]:
    if argument.startswith("--lines="):
        limit = int(argument.split("=", 1)[1])
        continue
    path, name = argument.split(":")
    old, new = body(OLD, path, name), body(NEW, path, name)
    same = old == new
    print(f"=== {name} ({path}) 66910c10 sha256 {hashlib.sha256(old.encode()).hexdigest()[:12]} "
          f"046accf6 sha256 {hashlib.sha256(new.encode()).hexdigest()[:12]} identical={same}")
    print("\n".join(old.splitlines()[:limit]))
    if not same:
        print("--- 046accf6:")
        print("\n".join(new.splitlines()[:limit]))
