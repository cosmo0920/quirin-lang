#!/usr/bin/env python3
"""Card K6: print the typed-path parity rows of a stream, without unit hashes.

usage: show_parity.py PARITY_JSONL [FUNCTION_SUBSTRING]
"""
import json
import sys

want = sys.argv[2] if len(sys.argv) > 2 else None
for line in open(sys.argv[1]):
    row = json.loads(line)
    if want and want not in json.dumps(row.get("function", "")):
        continue
    print(json.dumps({k: v for k, v in row.items() if k not in ("unit", "source_sha256")})[:600])
