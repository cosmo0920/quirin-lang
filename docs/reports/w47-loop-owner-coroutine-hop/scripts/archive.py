#!/usr/bin/env python3
"""archive.py SOURCE_DIR DEST_DIR NAME...: gzip each SOURCE_DIR/NAME into
DEST_DIR/NAME.gz (mtime 0, so the archive is reproducible) and write a
filename-keyed manifest (raw SHA-256 and size of each file)."""
import gzip
import hashlib
import json
import pathlib
import sys


def main():
    source, dest = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    dest.mkdir(parents=True, exist_ok=True)
    manifest = {}
    for name in sys.argv[3:]:
        data = (source / name).read_bytes()
        target = dest / (name + ".gz")
        with open(target, "wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", mtime=0, filename="") as out:
            out.write(data)
        manifest[name] = {"file": target.name, "sha256": hashlib.sha256(data).hexdigest(),
                          "bytes": len(data)}
    (dest / "manifest.json").write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    print(json.dumps(manifest, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
