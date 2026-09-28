#!/usr/bin/env python3
"""archive.py RUN_DIR OUT_DIR

Gzips every regular file of an arbiter run into OUT_DIR (deterministic gzip:
mtime 0, no name) and writes a filename-keyed manifest.json with each raw
file's SHA-256 and size, the format of the round-2b baseline archive.
"""
import gzip
import hashlib
import json
import pathlib
import sys


def main():
    run = pathlib.Path(sys.argv[1])
    out = pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=False)
    manifest = {}
    for path in sorted(p for p in run.iterdir() if p.is_file()):
        data = path.read_bytes()
        target = out / (path.name + ".gz")
        with open(target, "wb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as handle:
                handle.write(data)
        manifest[path.name] = {"file": target.name, "sha256": hashlib.sha256(data).hexdigest(),
                               "bytes": len(data)}
    (out / "manifest.json").write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    print(f"archived {len(manifest)} files into {out}")


if __name__ == "__main__":
    main()
