#!/usr/bin/env python3
"""Archive one arbiter run as a baseline in the layout `baseline_identity.py` reads.

Usage: archive.py RUN_DIR OUT_DIR
"""
import gzip
import hashlib
import json
import pathlib
import shutil
import sys

FILES = {
    "diagnostics": "resource-diagnostics.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "contracts": "resource-diagnostics.contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "source": "monkey-source.json",
}


def main():
    run, out = map(pathlib.Path, sys.argv[1:3])
    out.mkdir(parents=True, exist_ok=True)
    manifest = {}
    for label, name in FILES.items():
        raw = (run / name).read_bytes()
        target = out / f"{label}.json.gz"
        with open(target, "wb") as handle, gzip.GzipFile(fileobj=handle, mode="wb", mtime=0, filename="") as zipped:
            zipped.write(raw)
        manifest[label] = {
            "file": target.name,
            "raw_sha256": hashlib.sha256(raw).hexdigest(),
            "gzip_sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
            "raw_bytes": len(raw),
        }
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    shutil.copyfile(run / "run.json", out / "run.json")


if __name__ == "__main__":
    main()
