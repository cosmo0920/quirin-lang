import json, subprocess
from pathlib import Path
ROOT = Path('.').resolve()
m = json.loads(Path('docs/reports/docs-structure-compression/migration.json').read_text())
sc = m['source_commit']
orig = {s: subprocess.check_output(['git', 'show', f'{sc}:{s}']).decode() for s in m['sources']}
for row in m['parts']:
    p = ROOT / row['path']
    if not p.exists():
        print('MISSING', row['key'], row['path']); continue
    t = p.read_text(); s = f"<!-- preserved:{row['key']}:start -->\n"; e = f"<!-- preserved:{row['key']}:end -->\n"
    if t.count(s) != 1 or t.count(e) != 1:
        print('NOMARK', row['key'], row['path']); continue
    pay = t.split(s)[1].split(e)[0]
    for tr in reversed(row['transforms']):
        pay = pay.replace(tr['new'], tr['old'], 1)
    ok = pay == orig[row['source']][row['start']:row['end']]
    print('OK  ' if ok else 'EDIT', row['key'], row['path'])
