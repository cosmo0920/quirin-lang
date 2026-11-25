"""K4 group facts on the 11-24 report: for each K4 group, the census uses
of its matched leaves (11-07 facts) and the source text at the leaf, and the
release callees at `actual:*` leaves.

usage: groups.py ROWS-1124.jsonl CENSUS_ARTIFACTS_DIR PREPARED_TREE OUT.json
"""
import collections, gzip, json, re, sys

K4 = {'formal-copied', 'formal-handed-contracted', 'formal-handed-uncontracted', 'formal-not-handed'}
rows = [json.loads(l) for l in open(sys.argv[1])]
old = {}
with gzip.open(sys.argv[2].rstrip('/') + '/alias-leaves.jsonl.gz', 'rt') as f:
    next(f)
    for l in f:
        r = json.loads(l)
        old.setdefault((r['file'], r['function'], r['variable'], r['origin']), r)
src = sys.argv[3].rstrip('/') + '/'
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


uses = collections.defaultdict(collections.Counter)
callee_at_leaf = collections.defaultdict(collections.Counter)
samples = collections.defaultdict(list)
for r in rows:
    if r['status'] != 'Unknown':
        continue
    for d in r['ds']:
        if d['c'] != 'unknown_alias':
            continue
        m = re.search(r'`([^`]*)`', d['d'] or '')
        v = m.group(1) if m else '?'
        leaf = old.get((r['file'], r['fn'], v, d['o']))
        if not leaf or leaf['group'] not in K4:
            continue
        g = leaf['group']
        for u in leaf['facts'].get('uses', []) or ['(none)']:
            uses[g][u] += 1
        t = line(d['o']) if d['o'] else ''
        cm = re.match(r'(?:return\s+|\w+\s*=\s*)?(\w+)\s*\(', t)
        if cm:
            callee_at_leaf[g][cm.group(1)] += 1
        if g != 'formal-handed-uncontracted' and len(samples[g]) < 25:
            samples[g].append(f"{r['fn']} `{v}`: {t[:100]}")
res = dict(uses={g: c.most_common() for g, c in uses.items()},
           callee_at_leaf={g: c.most_common(30) for g, c in callee_at_leaf.items()},
           samples=samples)
json.dump(res, open(sys.argv[4], 'w'), indent=1)
for g in sorted(uses):
    print('==', g, uses[g].most_common(12))
    print('   callees at leaf', callee_at_leaf[g].most_common(15))
for g, s in samples.items():
    print('--', g)
    for x in s[:25]:
        print('   ', x)
