#!/usr/bin/env python3
"""body.py NAME [NAME...] -- CG22-CALLBACK-REGISTRY-CONTRACT (adapted from REFCOUNT-FAMILY-BLOCKING-CALLEES, adapted from REFCOUNT-REFERENCE-RETURNING-DECLARATIONS, adapted from REFCOUNT-RESOURCE-KIND, adapted from SLOT-MOVE-AND-INTERIOR-BORROW, from W43-ABI-ENUM-AND-CONST-PP, from CALLEE-RETENTION-CLASS1): print each named C
function's definition (signature through the matching closing brace) from
the two audited Fluent Bit trees, and say whether the two texts are
byte-identical. A reading aid: every contract is written from the printed
text, read in full. Searches lib/ src/ plugins/ include/ for a line where
NAME is followed by `(` and the definition's `{` follows before any `;`."""
import pathlib
import re
import subprocess
import sys

OLD = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source")
NEW = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source")


def definitions(root, name):
    out = subprocess.run(["grep", "-rnE", "--include=*.c", "--include=*.h",
                          r"(^|[^A-Za-z0-9_])%s[[:space:]]*\(" % re.escape(name),
                          "lib", "src", "plugins", "include"],
                         cwd=root, capture_output=True, text=True, check=False).stdout
    found = []
    for line in out.splitlines():
        path, number, _ = line.split(":", 2)
        lines = (root / path).read_text(errors="replace").splitlines()
        start = int(number) - 1
        # find the opening brace before any ';'
        i, text = start, ""
        while i < len(lines) and i < start + 12:
            text += lines[i] + "\n"
            if ";" in lines[i] and "{" not in lines[i]:
                break
            if "{" in lines[i]:
                break
            i += 1
        else:
            continue
        if "{" not in lines[i] if i < len(lines) else True:
            continue
        head = lines[start]
        if re.match(r"^\s*(return|if|while|for|else|\w+\s*=)", head) or head.strip().startswith(name + "(") and head.rstrip().endswith(";"):
            continue
        # walk braces
        depth, j, body = 0, start, []
        opened = False
        while j < len(lines):
            body.append(lines[j])
            depth += lines[j].count("{") - lines[j].count("}")
            if "{" in lines[j]:
                opened = True
            if opened and depth <= 0:
                break
            j += 1
        # include up to 3 preceding signature lines (return type on its own line)
        pre = []
        k = start - 1
        while k >= 0 and k >= start - 3 and lines[k].strip() and not lines[k].rstrip().endswith((";", "}", "*/")) and not lines[k].lstrip().startswith(("#", "//", "*", "/*")):
            pre.insert(0, lines[k])
            k -= 1
        found.append((path, start + 1, "\n".join(pre + body)))
    return found


for name in sys.argv[1:]:
    old = definitions(OLD, name)
    new = definitions(NEW, name)
    for path, line, text in old:
        twin = [t for p, _, t in new if p == path]
        same = "identical at 046accf6" if twin and twin[0] == text else (
            "DIFFERS at 046accf6" if twin else "ABSENT at 046accf6")
        print("==== %s  %s:%d  (%s)" % (name, path, line, same))
        print(text)
        if twin and twin[0] != text:
            print("---- 046accf6 text:")
            print(twin[0])
    if not old:
        print("==== %s  no definition found at 66910c10" % name)
