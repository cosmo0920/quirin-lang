#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card CONNECT-ADOPT-DECLARED-DEPENDENCIES: list the direct callees of named
C functions in one source file (the body-checked specifications' readings).
Usage: `python3 callees.py <file.c> NAME...`.
"""
import re, sys
src = open(sys.argv[1]).read()
names = sys.argv[2:]
kw = {"if", "while", "for", "switch", "return", "sizeof"}
for fn in names:
    m = re.search(r'^[a-z][^;\n]*\b' + fn + r'\s*\([^;{]*\)\s*\{', src, re.M)
    if not m:
        print(fn, "NOT FOUND"); continue
    i = m.end(); depth = 1
    while depth:
        c = src[i]
        depth += c == '{'
        depth -= c == '}'
        i += 1
    body = src[m.end():i]
    calls = sorted(set(re.findall(r'\b([A-Za-z_][A-Za-z0-9_]*)\s*\(', body)) - kw)
    print(fn, ":", " ".join(calls))
