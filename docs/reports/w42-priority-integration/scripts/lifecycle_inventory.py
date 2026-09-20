#!/usr/bin/env python3
"""Measure the existing field/teardown evidence; issue no lifecycle judgment."""

import collections
import gzip
import hashlib
import json
import pathlib
import sys


def main():
    path = pathlib.Path(sys.argv[1])
    stored = path.read_bytes()
    raw = gzip.decompress(stored) if path.suffix == ".gz" else stored
    artifact = json.loads(raw)
    rows = artifact["results"]
    identities = [(r["library"], r["file"], r["function"]) for r in rows]
    if len(identities) != len(set(identities)):
        raise ValueError("duplicate source-function identities")
    inventories = {}
    for library in sorted({r["library"] for r in rows}):
        selected = [r for r in rows if r["library"] == library]
        inventories[library] = {
            "functions": len(selected),
            "field_contract_statuses": dict(collections.Counter(r["status"] for r in selected)),
            "rows": [
                {k: r[k] for k in ("file", "function", "status", "reason", "reason_code") if k in r}
                for r in selected
            ],
        }
    output = {
        "artifact": str(path),
        "uncompressed_sha256": hashlib.sha256(raw).hexdigest(),
        "source_revision": artifact["source_revision"],
        "artifact_scope": artifact["scope"],
        "interpretation": "Field-contract statuses are not terminal lifecycle certificates; this inventory issues no safety judgment.",
        "inventories": inventories,
        "backend_teardown_records": [
            {k: r[k] for k in ("kind", "file", "function", "status") if k in r}
            for r in artifact["backend_teardown"]
        ],
    }
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
