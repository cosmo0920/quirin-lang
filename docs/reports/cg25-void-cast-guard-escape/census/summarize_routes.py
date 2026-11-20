"""Answering fallback rows per probed shape (route-census output)."""
import json
import os
import sys

here = os.path.dirname(os.path.abspath(__file__))
prefix = sys.argv[1] if len(sys.argv) > 1 else "routes-base"
shape = None
out = {}
for line in open(os.path.join(here, f"{prefix}.routes.jsonl")):
    d = json.loads(line)
    if "shape" in d:
        shape = d["shape"]
        out[shape] = []
        continue
    if d["answered"]:
        out[shape].append(f'{d["table"]}:{d["rule"]}')
status = {}
for line in open(os.path.join(here, f"{prefix}.tsv")):
    parts = line.rstrip("\n").split("\t")
    status[parts[0]] = parts[1]
for s, v in out.items():
    print(f"{s:48} {status.get(s, '?'):8} {sorted(set(v))}")
