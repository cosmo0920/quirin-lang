"""Per-block byte deltas of every edited range, from the specs and new_status.py."""
import importlib.util
import json
import subprocess
from pathlib import Path

BASE = '8c1f15eb'


def base_bytes(path, a, b):
    lines = subprocess.check_output(['git', 'show', f'{BASE}:{path}']).decode().split('\n')
    return len('\n'.join(lines[a - 1:b]).encode())


rows = []
for s in ['spec-gateway', 'spec-diagnostics', 'spec-buffers']:
    spec = json.loads(Path(f'docs/reports/docs-compact-9/artifacts/{s}.json').read_text())
    for e in spec['edits']:
        old = base_bytes(spec['file'], e['start'], e['end'])
        new = len(Path(e['with']).read_text().rstrip('\n').encode()) if e['with'] else 0
        # A deleted line also takes its newline with it.
        if not e['with']:
            old += 1
        rows.append((spec['file'], f"{e['start']}-{e['end']}", e.get('title', 'header'), old, new, new - old))
spec = importlib.util.spec_from_file_location('ns', 'docs/reports/docs-compact-9/scripts/new_status.py')
src = Path('docs/reports/docs-compact-9/scripts/new_status.py').read_text()
# Re-evaluate REPLACE without rewriting the file.
env = {}
exec(src.split('# Sanity:')[0], env)
for (a, b), text in sorted(env['REPLACE'].items()):
    old = base_bytes('docs/STATUS.md', a, b)
    new = len(text.encode())
    rows.append(('docs/STATUS.md', f'{a}-{b}', json.loads(Path('docs/reports/docs-compact-9/artifacts/status-titles.json').read_text())[f'{a}-{b}'][0], old, new, new - old))
for r in rows:
    print(f'{r[0]:32} {r[1]:9} {r[3]:6} {r[4]:6} {r[5]:+7}  {r[2]}')
