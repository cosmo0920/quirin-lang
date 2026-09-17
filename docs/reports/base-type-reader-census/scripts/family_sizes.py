#!/usr/bin/env python3
"""Where the wave-36 card's 18,555 came from, and what the archives hold."""
import gzip
import json
import pathlib
import re

INDIRECT = "indirect memory access is outside the resource projection"
ROOT = pathlib.Path(__file__).resolve().parents[2]


def counts(raw):
    doc = json.loads(raw)
    leaves = set()
    sole = set()
    sole_leaves = 0
    for unit in doc["units"]:
        key = (unit.get("library"), unit["file"])
        for function in unit["functions"]:
            families = {re.sub(r"`[^`]*`", "`X`", d["description"])
                        for d in function["diagnostics"]}
            here = [d for d in function["diagnostics"] if d["description"] == INDIRECT]
            for d in here:
                leaves.add(key + (function["function"], d["line"]))
            if here and families == {INDIRECT}:
                sole.add(key + (function["function"],))
                sole_leaves += len(here)
    return len(leaves), len(sole), sole_leaves


def main():
    for rel in [
        "docs/reports/c-gateway-constant-do-while/artifacts/diagnostics.json.gz",
        "docs/reports/c-gateway-constant-do-while/artifacts/diagnostics-before.json.gz",
        "docs/reports/joined-provenance-cycles/artifacts/diagnostics.json.gz",
        "docs/reports/joined-provenance-call-frames/artifacts/diagnostics-before.json.gz",
        "docs/reports/joined-provenance-call-frames/artifacts/diagnostics.json.gz",
        "docs/reports/w36-corpus-contracts-3/artifacts/diagnostics.json.gz",
        "docs/reports/w36-corpus-contracts-3/artifacts/diagnostics-before.json.gz",
    ]:
        path = ROOT / rel
        if not path.exists():
            print("missing                       ", rel)
            continue
        print("%-24s  %s" % (counts(gzip.decompress(path.read_bytes())), rel))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
