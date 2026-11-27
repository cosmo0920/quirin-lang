"""The census table: rows read, rows storing through a formal, pointer slots.

usage: counts.py <report.json> <census.json>

Counts distinct (unit, function) `Safe` rows of the report, the parent cards'
predicted `Safe` rows (CENSUS_ALSO_SAFE, as in run.sh) among its rows, the
distinct summary names any row reads that are not themselves read as `Safe`
rows, and from the census: rows storing through a formal, rows validating a
pointer out slot (`T **`), and those passing it across a call with an actual
the frame reads as reaching.
"""
import json
import os
import sys

EXCL = ('scalar', 'const', 'fnptr', 'null', 'string', 'addr-local')
ALSO = set(filter(None, os.environ.get('CENSUS_ALSO_SAFE', '').split(',')))
report, census = (json.load(open(p)) for p in sys.argv[1:3])
safe, also, summaries = set(), set(), set()
for u in report['units']:
    for f in u['functions']:
        key = (u['file'], f['function'])
        if f.get('status') == 'Safe':
            safe.add(key)
        elif f['function'] in ALSO:
            also.add(key)
        for name in (f.get('contract_dependencies') or {}).get('summaries') or []:
            summaries.add((u['file'], name))
summaries -= safe | also
rows = [r for r in census['rows'] if 'error' not in r]
slots = [r for r in rows if any(s['type'].replace(' ', '').endswith('**') for s in r['stores'])]
across = [r for r in slots
          if any(a['reach'] not in EXCL for c in r['calls'] for a in c['actuals'])]
print(json.dumps(dict(route=census['route'], safe_rows=len(safe), parent_predicted_safe=len(also),
                      summaries_read=len(summaries), unread=len(census['rows']) - len(rows),
                      storing=len(rows), pointer_slot=len(slots),
                      across_reaching_call=[r['function'] for r in across])))
