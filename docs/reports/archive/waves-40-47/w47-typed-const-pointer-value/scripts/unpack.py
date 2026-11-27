#!/usr/bin/env python3
"""Decompress an archived arbiter directory (``*.gz`` plus ``manifest.json``)
into a private directory and check every raw SHA-256 the manifest records.

Usage: unpack.py ARCHIVE_DIR OUT_DIR
"""
import gzip
import hashlib
import json
import pathlib
import shutil
import sys


def main() -> int:
    archive, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=False)
    manifest = json.loads((archive / "manifest.json").read_text())
    shutil.copyfile(archive / "manifest.json", out / "manifest.json")
    bad = 0
    for gz in sorted(archive.glob("*.gz")):
        raw = gzip.decompress(gz.read_bytes())
        name = gz.name[: -len(".gz")]
        (out / name).write_bytes(raw)
        entry = manifest.get(name) or manifest.get("files", {}).get(name)
        want = None
        if isinstance(entry, dict):
            want = entry.get("sha256") or entry.get("raw_sha256")
        elif isinstance(entry, str):
            want = entry
        got = hashlib.sha256(raw).hexdigest()
        status = "unchecked" if want is None else ("ok" if want == got else "MISMATCH")
        bad += status == "MISMATCH"
        print(f"{name}: {status} {got}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
