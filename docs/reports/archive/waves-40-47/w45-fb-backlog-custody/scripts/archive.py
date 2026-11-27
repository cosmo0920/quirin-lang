#!/usr/bin/env python3
"""Archive one arbiter run as a baseline directory (the w45-parenthesised-callee
integration layout): gzipped artifacts plus a manifest of raw and gzip digests.

Usage: archive.py RUN_DIR OUT_DIR
"""

import gzip
import hashlib
import json
import pathlib
import shutil
import sys

ARTIFACTS = {
    "diagnostics": ("resource-diagnostics.json", "diagnostics.json.gz"),
    "chunk-contracts": ("resource-diagnostics.chunk-contracts.json", "chunk-contracts.json.gz"),
    "contracts": ("resource-diagnostics.contracts.json", "contracts.json.gz"),
    "source-effects": ("resource-diagnostics.source-effects.json", "source-effects.json.gz"),
    "source": ("monkey-source.json", "source.json.gz"),
}


def main():
    run, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    manifest = {}
    for key, (name, archived) in ARTIFACTS.items():
        raw = (run / name).read_bytes()
        target = out / archived
        with open(target, "wb") as handle:
            with gzip.GzipFile(filename="", mode="wb", fileobj=handle, mtime=0) as compressed:
                compressed.write(raw)
        manifest[key] = {
            "file": archived,
            "raw_sha256": hashlib.sha256(raw).hexdigest(),
            "gzip_sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
            "raw_bytes": len(raw),
        }
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    shutil.copyfile(run / "run.json", out / "run.json")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
