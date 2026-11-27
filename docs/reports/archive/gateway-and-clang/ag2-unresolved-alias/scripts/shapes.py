"""AG2 sub-shape table from the census leaves (leaves.json written by census.py).

usage: shapes.py LEAVES.json OUT.json
"""
import collections, json, sys

L = json.load(open(sys.argv[1]))


def subshape(l):
    c, s = l['cls'], l['shape']
    if c == 'literal':
        return 'S3 literal copied into a local'
    if c == 'constant':
        return 'S2 enumeration constant copied or tested'
    if c == 'formal-pointer' and s == 'copy-source':
        return 'S1 pointer formal copied into a local'
    if c == 'formal-pointer':
        return 'S6 pointer formal used directly (' + s + ')'
    if c == 'local-pointer' and s == 'condition':
        return 'S4 local pointer in a condition'
    if c == 'local-pointer':
        return 'S5 local pointer read, handed or returned (' + s + ')'
    if c == 'global-or-undeclared':
        return 'S7 file-scope object or undeclared name (' + s + ')'
    if c.startswith('synthetic'):
        return 'S8 lowering placeholder ' + c.split(':', 1)[1]
    return 'S9 other (' + c + ')'


def family(name):
    return name.split(' (')[0]


rows = collections.defaultdict(list)
for l in L:
    l['sub'] = subshape(l)
    rows[(str(l['unit']), l['lib'], l['file'], l['fn'])].append(l)

alias_only = {k for k, v in rows.items() if v[0]['nother'] == 0}
table = {}
for key_fn, title in ((lambda l: family(l['sub']), 'family'), (lambda l: l['sub'], 'detail')):
    stats = collections.defaultdict(lambda: dict(leaves=0, primary=0, secondary=0, rows=set(),
                                                  rows_alone=set(), rows_alone_primary=set(), sample=[]))
    for k, v in rows.items():
        keys = {key_fn(l) for l in v}
        prim_keys = {key_fn(l) for l in v if l['cause'] == 'primary'}
        for l in v:
            st = stats[key_fn(l)]
            st['leaves'] += 1
            st[l['cause']] += 1
            st['rows'].add(k)
            if l['cause'] == 'primary' and len(st['sample']) < 6 and all(x['fn'] != l['fn'] for x in st['sample']):
                st['sample'].append(dict(fn=l['fn'], name=l['name'], origin=l['origin'], line=l['line']))
        if k in alias_only and len(keys) == 1:
            stats[next(iter(keys))]['rows_alone'].add(k)
    out = []
    for name, st in stats.items():
        out.append(dict(subshape=name, leaves=st['leaves'], primary=st['primary'], secondary=st['secondary'],
                        rows=len(st['rows']), rows_alone=len(st['rows_alone']),
                        rows_alone_list=sorted('::'.join(k) for k in st['rows_alone']), sample=st['sample']))
    out.sort(key=lambda r: (-r['rows_alone'], -r['primary'], r['subshape']))
    table[title] = out

summary = dict(
    leaves=len(L),
    primary=sum(1 for l in L if l['cause'] == 'primary'),
    secondary=sum(1 for l in L if l['cause'] == 'secondary'),
    secondary_why=collections.Counter(l['why'] for l in L if l['cause'] == 'secondary'),
    rows=len(rows),
    alias_only_rows=len(alias_only),
    rows_all_primary=sum(1 for v in rows.values() if all(l['cause'] == 'primary' for l in v)),
    rows_any_primary=sum(1 for v in rows.values() if any(l['cause'] == 'primary' for l in v)),
    rows_all_secondary=sum(1 for v in rows.values() if all(l['cause'] == 'secondary' for l in v)),
)
json.dump(dict(summary=summary, table=table), open(sys.argv[2], 'w'), indent=1, default=list)
print(json.dumps(summary, indent=1, default=list))
for r in table['family']:
    print(f"| {r['subshape']} | {r['leaves']} | {r['primary']} | {r['secondary']} | {r['rows']} | {r['rows_alone']} |")
print()
for r in table['detail'][:30]:
    print(f"| {r['subshape']} | {r['leaves']} | {r['primary']} | {r['secondary']} | {r['rows']} | {r['rows_alone']} |")
