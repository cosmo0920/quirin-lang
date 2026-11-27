"""Decompress an archived artifact directory into a private one and check manifest SHA-256s.

Usage: unpack.py ARCHIVE_DIR OUT_DIR
"""
import gzip, hashlib, json, pathlib, sys
src, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=False)
manifest = json.loads((src / 'manifest.json').read_text())
ok = True
for name, entry in sorted(manifest.items()):
    data = gzip.decompress((src / entry['file']).read_bytes())
    (out / name).write_bytes(data)
    good = hashlib.sha256(data).hexdigest() == entry['raw_sha256']
    ok &= good
    print(name, 'raw_sha256 ok' if good else 'MISMATCH')
sys.exit(0 if ok else 1)
