#!/usr/bin/env python3
"""Archive one audit run's five raw artifacts, gzip-compressed with mtime 0.

Usage: archive.py RUN_DIR OUT_DIR

Writes `<label>.json.gz` for each artifact, `manifest.json` with raw and gzip
SHA-256 and raw size, and a copy of the run's `run.json`. OUT_DIR must not
exist, so a published archive is never overwritten.
"""

import gzip
import hashlib
import json
import pathlib
import shutil
import sys

NAMES = {
    "diagnostics": "resource-diagnostics.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "contracts": "resource-diagnostics.contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "source": "monkey-source.json",
}


def main() -> int:
    source = pathlib.Path(sys.argv[1])
    out = pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=False)
    manifest = {}
    for label, name in NAMES.items():
        data = (source / name).read_bytes()
        packed = gzip.compress(data, mtime=0)
        target = out / f"{label}.json.gz"
        target.write_bytes(packed)
        manifest[label] = {
            "file": target.name,
            "raw_sha256": hashlib.sha256(data).hexdigest(),
            "gzip_sha256": hashlib.sha256(packed).hexdigest(),
            "raw_bytes": len(data),
        }
    shutil.copyfile(source / "run.json", out / "run.json")
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
