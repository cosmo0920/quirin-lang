#!/usr/bin/env python3
"""Archive one run directory's artifacts, gzipped deterministically, with a manifest.

Usage: archive.py RUN_DIR OUT_DIR [NAME ...]

Each named file (default: every regular file of RUN_DIR) is written to
OUT_DIR as NAME.gz with mtime 0. A `parity.jsonl*` stream is written with its
rows sorted, since worker arrival order is not part of the record; the
manifest names both the raw and the sorted digest. `manifest.json` records
raw SHA-256, raw bytes and gzip SHA-256 of every archived file.
"""

import gzip
import hashlib
import io
import json
import pathlib
import sys


def main():
    run, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    names = sys.argv[3:] or sorted(p.name for p in run.iterdir() if p.is_file())
    out.mkdir(parents=True, exist_ok=True)
    manifest = {}
    for name in names:
        raw = (run / name).read_bytes()
        entry = {"raw_sha256": hashlib.sha256(raw).hexdigest(), "raw_bytes": len(raw)}
        data = raw
        if name.startswith("parity.jsonl"):
            data = b"".join(sorted(raw.splitlines(keepends=True)))
            entry["sorted_rows_sha256"] = hashlib.sha256(data).hexdigest()
            entry["archived_form"] = "rows sorted"
        buffer = io.BytesIO()
        with gzip.GzipFile(filename=name, mode="wb", fileobj=buffer, mtime=0) as stream:
            stream.write(data)
        packed = buffer.getvalue()
        (out / (name + ".gz")).write_bytes(packed)
        entry["file"] = name + ".gz"
        entry["gzip_sha256"] = hashlib.sha256(packed).hexdigest()
        manifest[name] = entry
    (out / "manifest.json").write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    print(json.dumps({k: v["raw_sha256"] for k, v in manifest.items()}, indent=1))


if __name__ == "__main__":
    main()
