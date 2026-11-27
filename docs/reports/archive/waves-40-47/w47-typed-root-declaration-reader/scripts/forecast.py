"""Forecast candidates: functions with a premise-only refusal the AST reader could serve.

Usage: forecast.py PREMISE_CENSUS_JSON BEFORE_DIAGNOSTICS OUT_JSON
A function is a *direct candidate* when it is Unknown in BEFORE and every one of
its leaves is an unsupported COND/RET/IND leaf on the line of a servable
premise-only row (or up to six lines before one). Servable rows: lexical
reader unreadable (every hook), or a condition-assignment whose lexical
declared-local answer is false.
"""
import collections, json, sys
census = json.load(open(sys.argv[1]))
diagnostics = json.load(open(sys.argv[2]))
servable = collections.defaultdict(set)
for r in census['rows']:
    if not r['readable'] or (r['hook'] == 'condition-assignment' and r['declared'] is False):
        servable[r['function']].add(r['line'])


def family(d):
    desc = d['description']
    if d['category'] != 'unsupported':
        return d['category']
    if desc == 'indirect memory access is outside the resource projection':
        return 'IND'
    if desc.startswith('unmodeled call, aggregate or indirect access in return'):
        return 'RET'
    if desc.startswith('unmodeled call, aggregate or indirect access in'):
        return 'COND'
    return 'OTHER:' + desc[:60]


out = []
for unit in diagnostics['units']:
    for row in unit.get('functions', []):
        name = row['function']
        if name not in servable:
            continue
        lines = servable[name]
        leaves = row.get('diagnostics', [])
        blocking = []
        for d in leaves:
            fam = family(d)
            covered = fam in ('IND', 'RET', 'COND') and any(d['line'] + k in lines for k in range(0, 7))
            if not covered:
                blocking.append(f"{fam}@{d['line']}")
        out.append({'library': unit['library'], 'file': unit['file'], 'function': name,
                    'status': row['status'], 'servable_lines': sorted(lines),
                    'leaves': len(leaves), 'blocking': blocking,
                    'direct_candidate': row['status'] == 'Unknown' and not blocking})
json.dump(out, open(sys.argv[3], 'w'), indent=1)
print('servable functions (rows in units):', len(out))
print('status:', dict(collections.Counter(r['status'] for r in out)))
cands = [r for r in out if r['direct_candidate']]
print('direct candidates:', len(cands))
for r in cands:
    print('  ', r['library'], r['file'].split('/')[-1], r['function'], r['servable_lines'])
