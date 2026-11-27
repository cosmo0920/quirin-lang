"""Justify every function row a link map changes on the `current` route.

usage: import_justify.py MAP_FREE.json WITH_MAP.json CONTRACTS_JSON_WITH_MAP STDERR_LOG OUT.json

The two runs analyse every unit under the same contracts and refutation
table (the import phase is refused otherwise) and differ only by the
imports a caller holds, which the map run's progress lines list per unit
(`cross-unit scope ...: <held names>`). A changed row (status or diagnostic
leaves) is justified when its unit held at least one import and its status
did not move away from Safe. It is `direct` when a map-free leaf names a held
import, `consequential` otherwise (a leaf the import's summary resolves
indirectly, such as an alias or a branch condition over the call). Writes
every changed row and prints totals; exit 1 when any is unjustified.
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


def named(dependencies):
    names = set()
    for value in (dependencies or {}).values():
        if isinstance(value, list):
            names.update(v for v in value if isinstance(v, str))
        elif isinstance(value, dict):
            names.update(value)
    return names


a, b = units(sys.argv[1]), units(sys.argv[2])
held = {}
for line in open(sys.argv[4]):
    m = re.match(r'progress: \[\d+s\] cross-unit scope (\S+): .*?\(budget \d+\)(?:: (.*))?$', line.rstrip('\n'))
    if m:
        held[m.group(1)] = set((m.group(2) or '').split(', ')) - {''}
sidecar = json.load(open(sys.argv[3]))
link = sidecar.get('assumptions', sidecar).get('cross_unit_link_resolution') or {}
imported = {row['function'] for row in link.get('imported', [])}
rows = []
totals = collections.Counter()
unit_moves = collections.Counter()
for key in sorted(a):
    ua, ub = a[key], b[key]
    unit_moves[(ua['status'], ub['status'])] += 1
    fb = {f['function']: f for f in ub.get('functions', [])}
    for fa in ua.get('functions', []):
        rb = fb[fa['function']]
        if fa['status'] == rb['status'] and leaves(fa) == leaves(rb):
            continue
        reads = sorted(held.get(f'{key[0]}/{key[1]}', set()))
        worse = ORDER[rb['status']] > ORDER[fa['status']]
        ok = bool(reads) and not worse
        direct = any(name in text for _, text in leaves(fa) for name in reads)
        totals[(fa['status'], rb['status'],
                ('direct' if direct else 'consequential') if ok else 'UNJUSTIFIED')] += 1
        rows.append({
            'unit': f'{key[0]}/{key[1]}', 'function': fa['function'],
            'before': fa['status'], 'after': rb['status'],
            'unit_held_imports': reads,
            'direct': direct,
            'leaves_gone': sorted(map(list, (leaves(fa) - leaves(rb)).elements())),
            'leaves_new': sorted(map(list, (leaves(rb) - leaves(fa)).elements())),
            'justified': ok,
        })
json.dump({'imported_homes': len(imported), 'changed_rows': rows}, open(sys.argv[5], 'w'), indent=1)
print('units (before, after):', dict(unit_moves))
for key, count in sorted(totals.items()):
    print(count, *key)
sys.exit(0 if all(r['justified'] for r in rows) else 1)
