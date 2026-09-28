#!/usr/bin/env python3
"""Compare a run directory with a gzipped, file-keyed baseline archive.

Usage: identity.py RUN_DIR ARCHIVE_DIR OUT_JSON

For every file the archive's manifest.json names, the raw SHA-256 of the run's
file against the manifest's `raw_sha256`; for a differing JSON-lines stream,
also whether its sorted lines are identical. Exit 0 when the verdict-bearing
`resource-diagnostics.json` is byte-identical.
"""
import gzip
import hashlib
import json
import pathlib
import sys

run, archive, out = map(pathlib.Path, sys.argv[1:4])
manifest = json.loads((archive / 'manifest.json').read_text())
result = {}
for name, entry in sorted(manifest.items()):
    path = run / name
    if not path.exists():
        result[name] = 'missing-in-run'
        continue
    raw = path.read_bytes()
    row = {'raw_identical': hashlib.sha256(raw).hexdigest() == entry['raw_sha256']}
    if not row['raw_identical'] and name.endswith('.jsonl'):
        base = gzip.open(archive / entry['file']).read().splitlines()
        row['sorted_lines_identical'] = sorted(base) == sorted(raw.splitlines())
    result[name] = row
out.write_text(json.dumps(result, indent=1) + '\n')
print(json.dumps(result, indent=1))
sys.exit(0 if result.get('resource-diagnostics.json', {}).get('raw_identical') is True else 1)
