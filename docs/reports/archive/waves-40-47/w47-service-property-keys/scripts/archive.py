#!/usr/bin/env python3
"""Archive a run's files under the report's artifacts/, gzipped, with a manifest.

Usage: archive.py SOURCE_DIR DEST_DIR NAME...

Each NAME (a file in SOURCE_DIR) is written to DEST_DIR as NAME.gz (gzip
with no timestamp, so the bytes are reproducible) and listed in
DEST_DIR/manifest.json as {"NAME": {"file", "sha256" (of the raw bytes),
"bytes"}}. DEST_DIR must not exist.
"""

import gzip
import hashlib
import json
import pathlib
import sys


def main():
    source, dest = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    dest.mkdir(parents=True, exist_ok=False)
    manifest = {}
    for name in sys.argv[3:]:
        data = (source / name).read_bytes()
        (dest / f"{name}.gz").write_bytes(gzip.compress(data, mtime=0))
        manifest[name] = {"bytes": len(data), "file": f"{name}.gz",
                          "sha256": hashlib.sha256(data).hexdigest()}
    (dest / "manifest.json").write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
