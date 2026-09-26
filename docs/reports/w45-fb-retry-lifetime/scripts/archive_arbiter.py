#!/usr/bin/env python3
"""Archive one arbiter run as a baseline directory (the layout of
`docs/reports/w45-parenthesised-callee/integration/artifacts/`).

usage: archive_arbiter.py RUN_DIR DEST_DIR
"""
import gzip
import hashlib
import json
import pathlib
import sys

FILES = {
    "diagnostics": ("resource-diagnostics.json", "diagnostics.json.gz"),
    "chunk-contracts": ("resource-diagnostics.chunk-contracts.json", "chunk-contracts.json.gz"),
    "contracts": ("resource-diagnostics.contracts.json", "contracts.json.gz"),
    "source-effects": ("resource-diagnostics.source-effects.json", "source-effects.json.gz"),
    "source": ("monkey-source.json", "source.json.gz"),
}


def main():
    run, dest = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    dest.mkdir(parents=True, exist_ok=False)
    manifest = {}
    for key, (raw_name, packed_name) in FILES.items():
        raw = (run / raw_name).read_bytes()
        packed = gzip.compress(raw, 9, mtime=0)
        (dest / packed_name).write_bytes(packed)
        manifest[key] = {
            "file": packed_name,
            "raw_sha256": hashlib.sha256(raw).hexdigest(),
            "gzip_sha256": hashlib.sha256(packed).hexdigest(),
            "raw_bytes": len(raw),
        }
    (dest / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (dest / "run.json").write_bytes((run / "run.json").read_bytes())


if __name__ == "__main__":
    main()
