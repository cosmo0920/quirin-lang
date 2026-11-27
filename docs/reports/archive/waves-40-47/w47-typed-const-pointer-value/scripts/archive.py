#!/usr/bin/env python3
"""Archive a run directory: every file gzipped (mtime 0), with a manifest of
raw and gzip SHA-256s. Usage: archive.py RUN_DIR OUT_DIR"""
import gzip, hashlib, json, pathlib, sys

src_dir = pathlib.Path(sys.argv[1])
dst_dir = pathlib.Path(sys.argv[2])
dst_dir.mkdir(parents=True, exist_ok=True)
manifest = {}
for f in sorted(src_dir.iterdir()):
    if not f.is_file():
        continue
    if f.name == "manifest.json":
        continue
    data = f.read_bytes()
    raw_sha = hashlib.sha256(data).hexdigest()
    gz_path = dst_dir / (f.name + ".gz")
    gz_bytes = gzip.compress(data, mtime=0)
    gz_path.write_bytes(gz_bytes)
    gz_sha = hashlib.sha256(gz_bytes).hexdigest()
    manifest[f.name] = {
        "file": f.name + ".gz",
        "gzip_sha256": gz_sha,
        "raw_bytes": len(data),
        "raw_sha256": raw_sha,
    }
(dst_dir / "manifest.json").write_text(json.dumps(manifest, indent=1) + "\n")
print(f"archived {len(manifest)} files into {dst_dir}")
