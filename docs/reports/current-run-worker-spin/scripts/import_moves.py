"""Classify every function row that differs between a map-free `current`
report and a run with a cross-unit link map.

usage: import_moves.py MAP_FREE.json WITH_MAP.json CONTRACTS_JSON_WITH_MAP

A row is `import-explained` when its map-free leaves minus its map leaves
are only `call to uncontracted function F` for imported F (the summary now
applies) and its status does not move away from Safe. Every other differing
row is listed for review. Prints totals by (status before -> after, class).
"""
import collections
import json
import re
import sys

ORDER = {'Safe': 0, 'Pending': 1, 'Unknown': 2, 'Unsafe': 3}


def units(path):
    return {(u['library'], u['file']): u for u in json.load(open(path))['units']}


def leaves(row):
    return collections.Counter((d.get('category'), d.get('description'))
                               for d in row.get('diagnostics', []))


a, b = units(sys.argv[1]), units(sys.argv[2])
sidecar = json.load(open(sys.argv[3]))
link = sidecar.get('assumptions', sidecar).get('cross_unit_link_resolution') or {}
imported = {row['function'] for row in link.get('imported', [])}
print(f'imported homes: {len(imported)}')
totals = collections.Counter()
review = []
for key in sorted(a):
    ua, ub = a[key], b.get(key)
    if ub is None:
        review.append(f'{key}: unit missing with the map')
        continue
    if ua['status'] != ub['status']:
        totals[('unit', ua['status'], ub['status'])] += 1
    fb = {f['function']: f for f in ub.get('functions', [])}
    for fa in ua.get('functions', []):
        rb = fb.get(fa['function'])
        if rb is None:
            review.append(f'{key} {fa["function"]}: row missing with the map')
            continue
        la, lb = leaves(fa), leaves(rb)
        if fa['status'] == rb['status'] and la == lb:
            continue
        gone = la - lb
        explained = all(
            category == 'unsupported'
            and (m := re.fullmatch(r'call to uncontracted function `([^`]+)`', text))
            and m.group(1) in imported
            for (category, text) in gone)
        worse = ORDER.get(rb['status'], 9) > ORDER.get(fa['status'], 9)
        kind = 'import-explained' if explained and gone and not worse else 'review'
        totals[(fa['status'], rb['status'], kind)] += 1
        if kind == 'review':
            review.append(f'{key} {fa["function"]}: {fa["status"]} -> {rb["status"]}; '
                          f'gone {sorted(gone)}; new {sorted(lb - la)}')
for key, count in sorted(totals.items()):
    print(count, *key)
print(f'review rows: {len(review)}')
for line in review:
    print('  ' + line)
