"""The validated summaries `Safe` rows read, with their own rows' status.

usage: summaries_used.py <report.json>

Prints every name a `Safe` row lists under `contract_dependencies.summaries`
whose own row in the same unit is not `Safe` (those the AST census reads in
addition, through CENSUS_ALSO_SAFE), and the count of the rest.
"""
import collections as C
import json
import sys

d = json.load(open(sys.argv[1]))
status = {}
for u in d['units']:
    for f in u['functions']:
        status[(u['file'], f['function'])] = f.get('status')
used = C.Counter()
for u in d['units']:
    for f in u['functions']:
        if f.get('status') != 'Safe':
            continue
        for name in (f.get('contract_dependencies') or {}).get('summaries') or []:
            used[(u['file'], name)] += 1
not_safe = {k: v for k, v in used.items() if status.get(k) != 'Safe'}
print('summaries read by Safe rows', len(used), 'whose own row is not Safe', len(not_safe))
for (file, name), n in sorted(not_safe.items()):
    print(f'  {status.get((file, name))!s:8} {file} {name} (read by {n})')
print(','.join(sorted({name for _, name in not_safe})))
