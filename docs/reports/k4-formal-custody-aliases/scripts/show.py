"""Print the diagnostics of selected rows with their source lines.

usage: show.py ROWS.jsonl RECENSUS.json PREPARED_TREE
"""
import json, sys
rows = [json.loads(l) for l in open(sys.argv[1])]
rc = json.load(open(sys.argv[2]))
src = sys.argv[3].rstrip('/') + '/'
want = {(x['file'], x['fn']) for x in rc['k4_alone']} | {(x['file'], x['fn']) for x in rc['k4_alone_with_new']}
if len(sys.argv) > 4:
    want = {tuple(a.split('::')) for a in sys.argv[4:]}
cache = {}
def line(o):
    p, l = o.rsplit(':', 1)
    if p not in cache:
        try:
            cache[p] = open(src + p, errors='replace').read().split('\n')
        except OSError:
            cache[p] = []
    t = cache[p]
    l = int(l)
    return t[l - 1].strip() if 0 < l <= len(t) else '?'
seen = set()
for r in rows:
    k = (r['file'], r['fn'])
    if k not in want or r['status'] != 'Unknown' or k in seen:
        continue
    seen.add(k)
    print('==', r['lib'], r['file'], r['fn'], 'unit', r['u'])
    for d in r['ds']:
        print('   ', d['c'], '|', d['d'][:150], '|', d['o'], '|', line(d['o']) if d['o'] else '')
