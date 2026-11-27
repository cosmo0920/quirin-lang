"""List every refused out-slot store site of the current report with its source text."""
import json, re, sys

report, root, out = sys.argv[1:4]
d = json.load(open(report))
seen = {}
for u in d['units']:
    for f in u['functions']:
        ds = f.get('diagnostics', [])
        hits = [x for x in ds if 'without an out-parameter contract' in x['description']]
        if not hits:
            continue
        key = (u['file'], f['function'])
        row = seen.setdefault(key, dict(file=u['file'], lib=u['library'], function=f['function'],
                                         configs=0, other=set(), sites={}))
        row['configs'] += 1
        for x in ds:
            if x not in hits:
                row['other'].add(re.sub(r'`[^`]*`', 'X', x['description'])[:80])
        for x in hits:
            slot = re.search(r'`([^`]*)`', x['description']).group(1)
            origin = x.get('origin') or ''
            row['sites'].setdefault((slot, origin), 0)
            row['sites'][(slot, origin)] += 1

rows = []
for (file, function), row in sorted(seen.items()):
    sites = []
    for (slot, origin), n in sorted(row['sites'].items()):
        text = ''
        if ':' in origin:
            path, line = origin.rsplit(':', 1)
            try:
                lines = open(f'{root}/{path}', errors='replace').read().split('\n')
                text = lines[int(line) - 1].strip()
            except (OSError, ValueError, IndexError):
                text = '?'
        sites.append(dict(slot=slot, origin=origin, text=text))
    rows.append(dict(file=file, lib=row['lib'], function=function, configs=row['configs'],
                     other=sorted(row['other']), sites=sites))
json.dump(rows, open(out, 'w'), indent=1)
print(len(rows), sum(len(r['sites']) for r in rows))
