#!/usr/bin/env python3
"""Byte-compare the five artifacts of two run directories."""
import hashlib
import pathlib
import sys

NAMES = [
    "resource-diagnostics.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.source-effects.json",
    "monkey-source.json",
]
a, b = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
for name in NAMES:
    left, right = (a / name).read_bytes(), (b / name).read_bytes()
    print(name, "identical" if left == right else "DIFFERS",
          hashlib.sha256(left).hexdigest()[:16], hashlib.sha256(right).hexdigest()[:16])
