"""dabc_units.py RUN NAME[,NAME]: per unit, what its contract_body_evidence says
about each named declaration (validated by / failed by / unvalidated)."""
import collections
import json
import sys

run = sys.argv[1]
names = sys.argv[2].split(',')
report = json.load(open(run + '/resource-diagnostics.json'))
tally = collections.Counter()
for unit in report['units']:
    evidence = unit.get('contract_body_evidence') or {}
    for name in names:
        state = None
        if name in (evidence.get('failed') or {}):
            state = 'failed:' + ','.join(evidence['failed'][name])
        elif name in (evidence.get('validated') or {}):
            state = 'validated:' + ','.join(evidence['validated'][name])
        elif name in (evidence.get('unvalidated') or []):
            state = 'unvalidated'
        if state:
            tally[(name, state)] += 1
            if not state.startswith('validated') or '-v' in sys.argv:
                print(name, unit['library'], unit['file'], state)
for key, count in sorted(tally.items()):
    print(count, *key)
