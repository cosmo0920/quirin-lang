#!/usr/bin/env python3
"""Print each freed row's K7 leaf with its source line from the prepared tree
(read only). usage: sites.py RECENSUS.json SOURCE_ROOT [CONTEXT]"""
import json
import pathlib
import sys

d = json.load(open(sys.argv[1]))
root = pathlib.Path(sys.argv[2])
context = int(sys.argv[3]) if len(sys.argv) > 3 else 0
for r in sorted(d["freed_rows"] + d["loose_rows"], key=lambda r: (r["items"], r["file"])):
    print("#", r["function"], r["items"])
    for leaf in r["k7"]:
        path, line = leaf["origin"].rsplit(":", 1)
        text = (root / path).read_text(errors="replace").splitlines()
        n = int(line)
        for k in range(max(1, n - context), min(len(text), n + context) + 1):
            print(f"   {path}:{k}{'>' if k == n else ' '} {text[k - 1].rstrip()}")
