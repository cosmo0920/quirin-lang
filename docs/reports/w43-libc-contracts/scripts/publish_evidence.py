#!/usr/bin/env python3
"""Publish the arbiter's attribution and identity evidence beside the report.

Usage: publish_evidence.py BEFORE_DIR AFTER_DIR ATTRIBUTION.json INVENTORY.json

Writes, in this report directory: `attribution-summary.json` (the summary of
`attribute.py`'s output), `attribution.json.gz` (every leaf row), the full
callee inventory `inventory.json.gz`, and `after-identity.json` (raw SHA-256 of
all five artifacts BEFORE and AFTER, and the itemised difference of the one
non-verdict artifact that moves, `contracts`). Gzip members carry mtime 0.
"""

import gzip
import hashlib
import json
import pathlib
import sys

REPORT = pathlib.Path(__file__).resolve().parents[1]
NAMES = {
    "diagnostics": "resource-diagnostics.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "contracts": "resource-diagnostics.contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "source": "monkey-source.json",
}


def main() -> int:
    before, after = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    attribution = json.loads(pathlib.Path(sys.argv[3]).read_text())
    (REPORT / "attribution-summary.json").write_text(json.dumps(attribution["summary"], indent=1) + "\n")
    (REPORT / "attribution.json.gz").write_bytes(gzip.compress(json.dumps(attribution, indent=1).encode(), mtime=0))
    (REPORT / "inventory.json.gz").write_bytes(gzip.compress(pathlib.Path(sys.argv[4]).read_bytes(), mtime=0))
    identity = {}
    for label, name in NAMES.items():
        b, a = (before / name).read_bytes(), (after / name).read_bytes()
        identity[label] = {
            "before_sha256": hashlib.sha256(b).hexdigest(),
            "after_sha256": hashlib.sha256(a).hexdigest(),
            "byte_identical": a == b,
        }
    cb = json.loads((before / NAMES["contracts"]).read_text())
    ca = json.loads((after / NAMES["contracts"]).read_text())
    identity["contracts"]["difference"] = {
        "keys_differing": sorted(k for k in set(cb) | set(ca) if cb.get(k) != ca.get(k)),
        "sources_added": len(set(ca["sources"]) - set(cb["sources"])),
        "sources_removed": len(set(cb["sources"]) - set(ca["sources"])),
        "sources_changed": sum(1 for k in cb["sources"] if k in ca["sources"] and cb["sources"][k] != ca["sources"][k]),
    }
    (REPORT / "after-identity.json").write_text(json.dumps(identity, indent=1) + "\n")
    print(json.dumps(identity, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
