import json, re, sys, collections as C

d = json.load(open(sys.argv[1]))
out = sys.argv[2]
rows = []
for u in d['units']:
    for f in u['functions']:
        ds = f.get('diagnostics', [])
        hits = [x for x in ds if 'without an out-parameter contract' in x['description']]
        if not hits:
            continue
        classes = set()
        for x in ds:
            s = re.sub(r'`[^`]*`', 'X', x['description'])
            classes.add(x['category'] + ':' + s[:70])
        rows.append(dict(
            lib=u['library'], file=u['file'], unit_status=u['status'], fn=f['function'],
            fstatus=f.get('status'), n=len(hits),
            slots=sorted({re.search(r'`([^`]*)`', x['description']).group(1) for x in hits}),
            lines=sorted({x.get('origin') for x in hits if x.get('origin')}),
            ndiag=len(ds), classes=sorted(classes)))
json.dump(rows, open(out, 'w'), indent=1)
print(len(rows), sum(r['n'] for r in rows))
print(C.Counter(r['fstatus'] for r in rows))
print(C.Counter(r['file'].rsplit('.', 1)[-1] for r in rows))
sole = [r for r in rows if len(r['classes']) == 1]
print('sole', len(sole))
for r in sole:
    print(r['lib'], r['fn'], r['slots'], r['file'])
