"""K5 re-census: map the 11-07 census's alias groups onto the 11-24 report.

usage: recensus.py ROWS-1124.jsonl CENSUS_ARTIFACTS_DIR FROZEN_K5.md OUT.json

A leaf of the 11-24 report is matched to an 11-07 census leaf by
(file, function, variable, origin). Matched leaves take the 11-07 group;
unmatched ones are `new`. A row is freed by K5 alone when it is Unknown and
every diagnostic is an `unknown_alias` leaf whose group is a K5 group
(`new` leaves are counted separately as a ceiling: `alone_with_new`).
"""
import collections, gzip, json, re, sys

K5 = {'address-taken', 'borrowed-path-unguarded', 'contracted-result-unguarded', 'derived-local-unguarded',
      'field-store-owner', 'guarded-other', 'non-pointer-name', 'poisoned-by-refusal',
      'statement-expression-unguarded', 'uncontracted-result-unguarded'}

rows = [json.loads(l) for l in open(sys.argv[1])]
art = sys.argv[2].rstrip('/')
old = collections.defaultdict(list)
oldgroups = collections.Counter()
oldrows = {}
with gzip.open(art + '/alias-leaves.jsonl.gz', 'rt') as f:
    next(f)
    for l in f:
        r = json.loads(l)
        old[(r['file'], r['function'], r['variable'], r['origin'])].append(r)
        oldgroups[r['group']] += 1
frozen = set()
for l in open(sys.argv[3]):
    m = re.match(r'^(\S+)::(\S+)::(\S+)$', l.strip())
    if m:
        frozen.add((m.group(2), m.group(3)))


def name(desc):
    m = re.search(r'`([^`]*)`', desc or '')
    return m.group(1) if m else '?'


res = dict(old_groups=oldgroups)
groups = collections.Counter()
grows = collections.defaultdict(set)
alone = collections.Counter()
alone_rows = collections.defaultdict(list)
k5_alone = []
k5_alone_new = []
k5_rows = set()
status_by_fn = collections.defaultdict(set)
diag_by_fn = collections.defaultdict(list)
new_leaves = []
for r in rows:
    key = (r['file'], r['fn'])
    status_by_fn[key].add(r['status'])
    if r['status'] != 'Unknown':
        continue
    rid = (r['u'], r['file'], r['fn'])
    tags = []
    for d in r['ds']:
        if d['c'] == 'unknown_alias':
            v = name(d['d'])
            m = old.get((r['file'], r['fn'], v, d['o']))
            g = m[0]['group'] if m else 'new'
            groups[g] += 1
            grows[g].add(rid)
            tags.append('alias:' + g)
            if g == 'new':
                new_leaves.append(dict(lib=r['lib'], file=r['file'], fn=r['fn'], name=v, origin=d['o'], desc=d['d']))
        else:
            tags.append('other:' + d['c'])
    if any(t[6:] in K5 for t in tags if t.startswith('alias:')):
        k5_rows.add(rid)
    ts = set(tags)
    if ts and all(t.startswith('alias:') and t[6:] in K5 for t in ts):
        k5_alone.append(dict(lib=r['lib'], file=r['file'], fn=r['fn'], groups=sorted(t[6:] for t in ts), n=len(tags)))
    elif ts and all(t.startswith('alias:') and (t[6:] in K5 or t[6:] == 'new') for t in ts):
        k5_alone_new.append(dict(lib=r['lib'], file=r['file'], fn=r['fn'], groups=sorted(t[6:] for t in ts), n=len(tags),
                                 new=[(d['o'], name(d['d'])) for d in r['ds']]))
    if ts and all(t.startswith('alias:') for t in ts) and len({t for t in ts}) == 1:
        alone[next(iter(ts))] += 1
        alone_rows[next(iter(ts))].append(r['fn'])

frozen_now = {}
for k in sorted(frozen):
    file, fn = k
    frozen_now['::'.join(k)] = sorted(status_by_fn.get(k, {'absent'}))

# per-frozen current blockers
blk = {}
for r in rows:
    k = (r['file'], r['fn'])
    if k in frozen and r['status'] != 'Safe':
        c = collections.Counter()
        for d in r['ds']:
            if d['c'] == 'unknown_alias':
                v = name(d['d'])
                m = old.get((r['file'], r['fn'], v, d['o']))
                c['alias:' + (m[0]['group'] if m else 'new')] += 1
            else:
                c[d['c'] + ':' + re.sub(r'`[^`]*`', '`_`', d['d'] or '')[:70]] += 1
        blk.setdefault('::'.join(k), []).append(dict(u=r['u'], status=r['status'], blockers=c))

res.update(
    groups_1124=groups,
    group_rows_1124={g: len(v) for g, v in grows.items()},
    k5_leaves_1124=sum(v for g, v in groups.items() if g in K5),
    k5_rows_touched=len(k5_rows),
    k5_alone_rows=len(k5_alone),
    k5_alone_distinct=len({(x['file'], x['fn']) for x in k5_alone}),
    k5_alone=k5_alone,
    k5_alone_with_new_rows=len(k5_alone_new),
    k5_alone_with_new=k5_alone_new,
    alone_by_single_tag=alone,
    frozen_status_1124=frozen_now,
    frozen_blockers_1124=blk,
    new_leaf_count=len(new_leaves),
)
json.dump(res, open(sys.argv[4], 'w'), indent=1, default=list)
json.dump(new_leaves, open(sys.argv[4] + '.new-leaves.json', 'w'), indent=0)
for k in ('old_groups', 'groups_1124', 'group_rows_1124', 'k5_leaves_1124', 'k5_rows_touched', 'k5_alone_rows',
          'k5_alone_distinct', 'k5_alone_with_new_rows', 'alone_by_single_tag', 'new_leaf_count'):
    print(k, json.dumps(res[k], default=list))
print('frozen', collections.Counter(tuple(v) for v in frozen_now.values()))
