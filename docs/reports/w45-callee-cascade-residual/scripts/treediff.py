#!/usr/bin/env python3
"""treediff.py DIR... -- W45-CALLEE-CASCADE-RESIDUAL: which C sources under each
DIR differ between the two audited Fluent Bit trees (66910c10, the read-only
corpus tree, and 046accf6, the prepared system tree). Read only."""
import filecmp
import pathlib
import sys

OLD = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source")
NEW = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source")

for d in sys.argv[1:]:
    diffs = []
    for p in sorted((OLD / d).rglob("*")):
        if p.suffix not in (".c", ".h") or not p.is_file():
            continue
        q = NEW / p.relative_to(OLD)
        if not q.exists():
            diffs.append(("missing", str(p.relative_to(OLD))))
        elif not filecmp.cmp(p, q, shallow=False):
            diffs.append(("diff", str(p.relative_to(OLD))))
    print(d, len(diffs))
    for row in diffs[:40]:
        print("  ", *row)
