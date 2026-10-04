#!/usr/bin/env python3
"""defs.py NAME... -- CALLEE-RETENTION-CLASS1: every definition site of
each name in the 66910c10 tree (lib/ src/ plugins/ include/), lexically: a
line where NAME is followed by `(` and the next `{` or `;` is a `{`. A name
with more than one definition is a name-collision risk for a name-keyed
declaration and is checked by hand."""
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source")

for name in sys.argv[1:]:
    out = subprocess.run(["grep", "-rnE", "--include=*.c", "--include=*.h",
                          r"(^|[^A-Za-z0-9_])%s[[:space:]]*\(" % re.escape(name),
                          "lib", "src", "plugins", "include"],
                         cwd=ROOT, capture_output=True, text=True, check=False).stdout
    sites = []
    for line in out.splitlines():
        path, number, text = line.split(":", 2)
        if re.match(r"^\s*(return\b|if\b|while\b|for\b|\w+\s*=|\(|\*)", text) or "=" in text.split(name)[0]:
            continue
        lines = (ROOT / path).read_text(errors="replace").splitlines()
        tail = "\n".join(lines[int(number) - 1:int(number) + 12])
        rest = tail[tail.index(name):]
        brace, semi = rest.find("{"), rest.find(";")
        if brace != -1 and (semi == -1 or brace < semi):
            sites.append("%s:%s" % (path, number))
    print("%-40s %d  %s" % (name, len(sites), " ".join(sites[:6])))
