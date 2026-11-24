"""Read the user's `current` report (read-only) into a compact JSON-lines file.

usage: extract.py REPORT.json OUT.jsonl
One line per function row: unit index, library, file, function, status, diagnostics.
"""
import json, sys, collections
d = json.load(open(sys.argv[1]))
units = d['units']
cats = collections.Counter()
st = collections.Counter()
with open(sys.argv[2], 'w') as out:
    for i, u in enumerate(units):
        for f in u.get('functions', []):
            ds = [dict(c=x.get('category'), d=x.get('description'), l=x.get('line'), o=x.get('origin')) for x in f.get('diagnostics', [])]
            for x in ds:
                cats[(f['status'], x['c'])] += 1
            st[f['status']] += 1
            out.write(json.dumps(dict(u=i, lib=u.get('library'), file=u.get('file'), ustatus=u.get('status'),
                                      fn=f['function'], status=f['status'], ds=ds)) + '\n')
print('units', len(units), 'unit keys', [k for k in units[0] if k != 'functions'])
print('rows', dict(st))
for k, v in sorted(cats.items(), key=lambda kv: -kv[1])[:40]:
    print(k, v)
