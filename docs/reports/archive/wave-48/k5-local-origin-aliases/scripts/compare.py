"""Compare two `current` reports function row by function row.

usage: compare.py BEFORE.json AFTER.json [--units UNITS.txt] [--json OUT.json]

Rows are keyed by (unit file, configuration ordinal of that file, function).
With --units, only rows of the listed files are compared (a unit-scoped
reproduction against the full run). Prints status moves, rows whose
diagnostics changed, and the unknown_alias leaves removed and added.
"""
import collections, json, re, sys


def rows(path, only=None):
    d = json.load(open(path))
    seen = collections.Counter()
    out = {}
    for u in d['units']:
        f = u['file']
        if only is not None and f not in only:
            continue
        n = seen[f]
        seen[f] += 1
        for fn in u.get('functions', []):
            diags = sorted((x['category'], x['description'], x.get('origin') or '') for x in fn['diagnostics'])
            out[(f, n, fn['function'])] = (fn['status'], diags, u['library'])
    return out


def main():
    args = sys.argv[1:]
    only = None
    out_json = None
    if '--units' in args:
        i = args.index('--units')
        only = {l.strip() for l in open(args[i + 1]) if l.strip()}
        del args[i:i + 2]
    if '--json' in args:
        i = args.index('--json')
        out_json = args[i + 1]
        del args[i:i + 2]
    before, after = rows(args[0], only), rows(args[1], only)
    keys = sorted(set(before) | set(after))
    moves = collections.Counter()
    moved, changed, unmatched = [], [], []
    removed, added = collections.Counter(), collections.Counter()
    for k in keys:
        if k not in before or k not in after:
            unmatched.append(k)
            continue
        (sb, db, lib), (sa, da, _) = before[k], after[k]
        if sb != sa:
            moves[(sb, sa)] += 1
            moved.append(dict(library=lib, file=k[0], configuration=k[1], function=k[2], before=sb, after=sa,
                              removed=[x for x in db if x not in da], added=[x for x in da if x not in db]))
        elif db != da:
            changed.append(dict(library=lib, file=k[0], configuration=k[1], function=k[2], status=sb,
                                removed=[x for x in db if x not in da], added=[x for x in da if x not in db]))
        for x in db:
            if x not in da and x[0] == 'unknown_alias':
                removed[re.sub(r'`.*`', 'X', x[1])] += 1
        for x in da:
            if x not in db:
                added[x[0] + ':' + re.sub(r'`[^`]*`', 'X', x[1])] += 1
    totals = lambda r: dict(collections.Counter(v[0] for v in r.values()))
    result = dict(rows_before=len(before), rows_after=len(after), totals_before=totals(before),
                  totals_after=totals(after), moves={f'{a} -> {b}': n for (a, b), n in moves.items()},
                  moved=moved, content_changed=changed, unmatched=[list(k) for k in unmatched],
                  alias_leaves_removed=sum(removed.values()), diagnostics_added=dict(added))
    if out_json:
        json.dump(result, open(out_json, 'w'), indent=1)
    print(json.dumps({k: v for k, v in result.items() if k not in ('moved', 'content_changed')}, indent=1))
    for m in moved:
        print('MOVE', m['before'], '->', m['after'], m['library'], m['file'], m['function'],
              'removed', len(m['removed']), 'added', m['added'])
    print('content-changed rows', len(changed))
    for c in changed[:40]:
        print('  CHG', c['status'], c['file'], c['function'], 'removed', len(c['removed']), 'added', c['added'][:3])


main()
