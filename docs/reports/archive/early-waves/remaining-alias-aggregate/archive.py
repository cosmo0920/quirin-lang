#!/usr/bin/env python3
"""Archive the AFTER 185-unit artifacts of card ALIAS-AGGREGATE with a manifest.

Card NEIGHBOURS' archiver, reused apart from its output directory.
"""

import gzip
import hashlib
import json
import pathlib

SRC = pathlib.Path("measure/runs/after")
DST = pathlib.Path("docs/reports/remaining-alias-aggregate/artifacts")
PAIRS = [
    ("diagnostics", "resource-diagnostics.json"),
    ("contracts", "resource-diagnostics.contracts.json"),
    ("chunk-contracts", "resource-diagnostics.chunk-contracts.json"),
    ("source-effects", "resource-diagnostics.source-effects.json"),
    ("source", "monkey-source.json"),
]


def main() -> None:
    DST.mkdir(parents=True, exist_ok=True)
    manifest = {}
    for label, produced in PAIRS:
        data = (SRC / produced).read_bytes()
        out = DST / f"{label}.json.gz"
        with open(out, "wb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0, compresslevel=9) as fh:
                fh.write(data)
        manifest[label] = {
            "archived_path": f"{DST}/{label}.json.gz",
            "produced_file": produced,
            "uncompressed_bytes": len(data),
            "uncompressed_sha256": hashlib.sha256(data).hexdigest(),
        }
    (DST / "manifest.json").write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    for label, record in manifest.items():
        print(label, record["uncompressed_sha256"][:16], record["uncompressed_bytes"])


if __name__ == "__main__":
    main()
