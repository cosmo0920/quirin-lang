"""The committed census artifacts: one row per leaf and the per-form counts.

Usage: compact.py SHAPES.json OUT_DIR
"""
import collections as C, json, os, sys

rows = json.load(open(sys.argv[1]))
out = sys.argv[2]
os.makedirs(out, exist_ok=True)
with open(os.path.join(out, 'census.jsonl'), 'w') as f:
    for r in sorted(rows, key=lambda r: (r['unit'], r['fn'], r['line'], r['form'])):
        f.write(json.dumps({
            'unit': r['unit'], 'function': r['fn'], 'line': r['line'], 'origin': r['origin'],
            'form': r['form'], 'function_status': r['fstatus'],
            'other_families': len(r['others']),
            'statements': [{'texts': s[0], 'reading': s[1:]} for s in r['shapes']],
        }, sort_keys=True) + '\n')
summary = {}
for form in ['ASSIGN', 'DEST', 'BRACE', 'INCDEC', 'SIZEOF']:
    c = C.Counter()
    leaves = 0
    for r in rows:
        if r['form'] != form:
            continue
        leaves += 1
        if not r['shapes']:
            c['(no final refusal recorded)'] += 1
        for s in r['shapes']:
            c[' / '.join(x for x in s[1:] if x)] += 1
    summary[form] = {'leaves': leaves, 'shapes': dict(c.most_common())}
json.dump(summary, open(os.path.join(out, 'census-summary.json'), 'w'), indent=1, sort_keys=True)
print({k: v['leaves'] for k, v in summary.items()})
