#!/usr/bin/env python3
"""show_sites.py TOP30.json [BEFORE AFTER] -- NULL-SAFETY-PROPERTY: print each
candidate's source lines (the origin's file:line in the corpus tree, BEFORE
lines before and AFTER lines after) for the hand reading."""
import json
import os
import sys

ROOT = "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"
MONKEY = "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/monkey/monkey-source"
before = int(sys.argv[2]) if len(sys.argv) > 2 else 8
after = int(sys.argv[3]) if len(sys.argv) > 3 else 3
index = {}
for root in (ROOT, MONKEY):
    for directory, _, names in os.walk(root):
        for name in names:
            if name.endswith((".c", ".h")):
                path = os.path.join(directory, name)
                index.setdefault(os.path.relpath(path, root), path)
                index.setdefault(name, path)
for number, entry in enumerate(json.load(open(sys.argv[1])), 1):
    origin = entry.get("origin") or ""
    file, _, line = origin.rpartition(":")
    print(f"=== {number}. {entry['function']} {origin} {entry['subject']} {entry['sources']}")
    path = index.get(file) or index.get(os.path.basename(file))
    if not path or not line.isdigit():
        print("   <source not found>")
        continue
    lines = open(path, errors="replace").read().splitlines()
    at = int(line)
    for n in range(max(1, at - before), min(len(lines), at + after) + 1):
        print(f"{'>' if n == at else ' '}{n:5} {lines[n - 1]}")
