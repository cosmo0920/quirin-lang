"""Row-by-row comparison of two `current` reports (resource-diagnostics.json).

usage: compare.py A.json B.json

Prints, per unit (library/file, in report order): a unit status or reason
change, every function whose status or diagnostic leaves differ, and which
other keys of the unit or function rows differ (evidence fields such as
`contract_dependencies` or `cross_unit_link_resolution`). Exit 0 when no
unit status, unit reason, function status or diagnostic leaf differs.
"""
import json
import sys


def units(path):
    data = json.load(open(path))
    return {(u['library'], u['file']): u for u in data['units']}


def leaves(row):
    return sorted((d.get('category'), d.get('description'), d.get('line'))
                  for d in row.get('diagnostics', []))


a, b = units(sys.argv[1]), units(sys.argv[2])
verdict_rows = 0
evidence = {}
for key in sorted(set(a) | set(b)):
    ua, ub = a.get(key), b.get(key)
    if ua is None or ub is None:
        print(f'{key}: only in {"B" if ua is None else "A"}')
        verdict_rows += 1
        continue
    if (ua['status'], ua.get('reason')) != (ub['status'], ub.get('reason')):
        print(f'{key}: unit {ua["status"]} -> {ub["status"]}; reason {ua.get("reason")!r} -> {ub.get("reason")!r}')
        verdict_rows += 1
    for field in sorted(set(ua) | set(ub)):
        if field not in ('functions', 'status', 'reason') and ua.get(field) != ub.get(field):
            evidence[f'unit.{field}'] = evidence.get(f'unit.{field}', 0) + 1
    fa = {f['function']: f for f in ua.get('functions', [])}
    fb = {f['function']: f for f in ub.get('functions', [])}
    for name in sorted(set(fa) | set(fb)):
        ra, rb = fa.get(name), fb.get(name)
        if ra is None or rb is None:
            print(f'{key} {name}: row only in {"B" if ra is None else "A"}')
            verdict_rows += 1
            continue
        if ra['status'] != rb['status'] or leaves(ra) != leaves(rb):
            verdict_rows += 1
            print(f'{key} {name}: {ra["status"]} -> {rb["status"]}')
            for leaf in sorted(set(map(tuple, leaves(ra))) - set(map(tuple, leaves(rb)))):
                print(f'    - {leaf}')
            for leaf in sorted(set(map(tuple, leaves(rb))) - set(map(tuple, leaves(ra)))):
                print(f'    + {leaf}')
        for field in sorted(set(ra) | set(rb)):
            if field not in ('status', 'diagnostics') and ra.get(field) != rb.get(field):
                evidence[f'function.{field}'] = evidence.get(f'function.{field}', 0) + 1
print(f'units {len(a)} / {len(b)}; verdict or leaf differences: {verdict_rows}')
print('evidence-field differences (rows):', json.dumps(evidence, sort_keys=True))
sys.exit(1 if verdict_rows else 0)
