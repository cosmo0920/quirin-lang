#!/usr/bin/env python3
"""Every reconciliation `verify` claim on docs/STATUS.md, and whether it holds.

Usage: claims.py [REV]   (default: the working tree)

`contains`/`absent` are checked with the sweep's own normalisation, so each
line matches what `reconciliation_sweep.py --check` says for that one claim.
"""
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, 'scripts')
import reconciliation_sweep as sweep  # noqa: E402

TARGET = 'docs/STATUS.md'


def claims(obj):
    if isinstance(obj, dict):
        if 'file' in obj and ('contains' in obj or 'absent' in obj):
            yield obj
        for v in obj.values():
            yield from claims(v)
    elif isinstance(obj, list):
        for v in obj:
            yield from claims(v)


if len(sys.argv) > 1:
    text = subprocess.check_output(['git', 'show', f'{sys.argv[1]}:{TARGET}']).decode()
else:
    text = Path(TARGET).read_text()
norm = sweep._normalise(text)
rows = []
for rec in sorted(Path('docs/reports').glob('*/reconciliation.json')):
    for c in claims(json.loads(rec.read_text())):
        if c['file'] != TARGET:
            continue
        if 'contains' in c:
            ok, kind = sweep._normalise(c['contains']) in norm, 'contains'
        else:
            ok, kind = sweep._normalise(c['absent']) not in norm, 'absent'
        rows.append((rec.parent.name, kind, ok, c.get('contains') or c.get('absent')))
for r in rows:
    print('OK  ' if r[2] else 'FAIL', r[0], r[1], repr(r[3][:120]))
print(len(rows), 'claims;', sum(1 for r in rows if not r[2]), 'failing')
