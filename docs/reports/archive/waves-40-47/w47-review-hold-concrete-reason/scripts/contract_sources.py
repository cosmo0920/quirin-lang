#!/usr/bin/env python3
"""contract_sources.py BASE RUN -- show how the first unit's
`compile_configuration.dependencies.resource_contract_sources` differs
between two measurement directories (raw or `.gz`)."""
import gzip
import json
import pathlib
import sys


def load(directory):
    directory = pathlib.Path(directory)
    path = directory / "resource-diagnostics.json"
    data = path.read_bytes() if path.is_file() else gzip.decompress(
        (directory / "resource-diagnostics.json.gz").read_bytes())
    return json.loads(data)


base, run = load(sys.argv[1]), load(sys.argv[2])
b = base["units"][0]["compile_configuration"]["dependencies"]["resource_contract_sources"]
r = run["units"][0]["compile_configuration"]["dependencies"]["resource_contract_sources"]
print(type(b).__name__, len(b), len(r))
if isinstance(b, dict):
    for key in sorted(set(b) | set(r)):
        if b.get(key) != r.get(key):
            print(" ", key, "|", json.dumps(b.get(key))[:200], "|", json.dumps(r.get(key))[:200])
elif isinstance(b, list):
    for x, y in zip(b, r):
        if x != y:
            print(" ", json.dumps(x)[:300], "|", json.dumps(y)[:300])
else:
    print(json.dumps(b)[:300], json.dumps(r)[:300])
