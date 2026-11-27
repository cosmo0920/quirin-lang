"""K4 co-occurrence on the 11-24 report: for each Unknown row carrying a K4
leaf, the families of its other blockers.

usage: cooccur.py ROWS-1124.jsonl CENSUS_ARTIFACTS_DIR OUT.json
Families: `alias:<group>` (census group of the leaf, `new` if unmatched),
`UNC-RUN` (call to an uncontracted function the run defines), `UNC-EXT`
(other uncontracted call), `IND`, `COND`, `RET`, else the category.
A row is freed by K4 plus a set X when every blocker is a K4 leaf or in X.
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
defined = {r['fn'] for r in rows}


def name(desc):
    m = re.search(r'`([^`]*)`', desc or '')
    return m.group(1) if m else '?'


def fam(r, d):
    c, desc = d['c'], d['d'] or ''
    if c == 'unknown_alias':
        m = old.get((r['file'], r['fn'], name(desc), d['o']))
        return 'alias:' + (m['group'] if m else 'new')
    if desc.startswith('call to uncontracted function'):
        return 'UNC-RUN' if name(desc) in defined else 'UNC-EXT'
    if desc == 'indirect memory access is outside the resource projection':
        return 'IND'
    if desc == 'unmodeled call, aggregate or indirect access in return expression':
        return 'RET'
    if desc.startswith('unmodeled call, aggregate or indirect access in'):
        return 'COND'
    return 'other:' + c


touched = []
for r in rows:
    if r['status'] != 'Unknown':
        continue
    fams = [fam(r, d) for d in r['ds']]
    if any(f[6:] in K4 for f in fams if f.startswith('alias:')):
        touched.append((r, fams))
rest = collections.Counter()
single = collections.Counter()
for r, fams in touched:
    others = frozenset(f for f in fams if not (f.startswith('alias:') and f[6:] in K4))
    rest[','.join(sorted(others)) or '(none)'] += 1
    for f in others:
        single[f] += 1
alias_other = {'alias:' + g for g in ('poisoned-by-refusal', 'non-pointer-name', 'derived-local-unguarded',
               'borrowed-path-unguarded', 'uncontracted-result-unguarded', 'guarded-other',
               'contracted-result-unguarded', 'address-taken', 'statement-expression-unguarded',
               'field-store-owner', 'new', 'peer-release', 'unclassified', 'global-pointer')}
scen = {
    'K4 alone': set(),
    'K4 + UNC-RUN': {'UNC-RUN'},
    'K4 + other alias': alias_other,
    'K4 + IND': {'IND'},
    'K4 + IND + other alias': {'IND'} | alias_other,
    'K4 + UNC-RUN + IND + COND + RET + other alias': {'UNC-RUN', 'IND', 'COND', 'RET'} | alias_other,
}
freed = {k: sum(1 for r, fams in touched
                if all((f.startswith('alias:') and f[6:] in K4) or f in v for f in fams)) for k, v in scen.items()}
per_group_alone = collections.Counter()
for r, fams in touched:
    gs = {f[6:] for f in fams if f.startswith('alias:') and f[6:] in K4}
    if all(f.startswith('alias:') and f[6:] in K4 for f in fams) and len(gs) == 1:
        per_group_alone[next(iter(gs))] += 1
res = dict(rows_touched=len(touched), freed_alone_by_single_group=per_group_alone,
           others_by_family=single.most_common(), top_other_sets=rest.most_common(25), freed=freed)
json.dump(res, open(sys.argv[3], 'w'), indent=1)
print(json.dumps(res, indent=1))
