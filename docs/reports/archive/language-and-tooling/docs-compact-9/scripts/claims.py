"""Every reconciliation `verify` claim whose file this card touches, and whether it holds now.

`contains` is checked with the sweep's own normalisation, so the result matches
what `reconciliation_sweep.py --check` would say for that one claim.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, 'scripts')
import reconciliation_sweep as sweep  # noqa: E402

TOUCHED = {'AGENTS.md', 'CLAUDE.md', 'README.md', 'docs/STATUS.md',
           'docs/agents/gateway.md', 'docs/agents/diagnostics.md',
           'docs/agents/buffers-records.md', 'docs/agents/operations.md',
           'docs/agents/README.md', 'docs/development/increments/README.md'}
TOUCHED |= {f'docs/development/increments/{n}' for n in
            ('wave-45.md', 'wave-44.md', 'wave-41.md', 'wave-41-continuation.md')}


def claims(obj):
    if isinstance(obj, dict):
        if 'file' in obj and ('contains' in obj or 'absent' in obj):
            yield obj
        for v in obj.values():
            yield from claims(v)
    elif isinstance(obj, list):
        for v in obj:
            yield from claims(v)


rows = []
for rec in sorted(Path('docs/reports').glob('*/reconciliation.json')):
    data = json.loads(rec.read_text())
    for c in claims(data):
        if c['file'] not in TOUCHED:
            continue
        text = Path(c['file']).read_text() if Path(c['file']).exists() else ''
        norm = sweep._normalise(text)
        if 'contains' in c:
            ok = sweep._normalise(c['contains']) in norm
            kind = 'contains'
        else:
            ok = sweep._normalise(c['absent']) not in norm
            kind = 'absent'
        rows.append((str(rec.parent.name), c['file'], kind, ok, (c.get('contains') or c.get('absent'))[:90]))
for r in rows:
    print('OK  ' if r[3] else 'FAIL', r[0], r[1], r[2], repr(r[4]))
print(len(rows), 'claims;', sum(1 for r in rows if not r[3]), 'failing')
