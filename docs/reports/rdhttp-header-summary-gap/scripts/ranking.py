"""dabc_rank.py RUN [--all]: rank non-libc declared-assumption and partly-validated
declarations by corpus Safe rows, with the reason each body is not validated."""
import collections
import json
import sys

run = sys.argv[1]
show_all = '--all' in sys.argv
sidecar = json.load(open(run + '/resource-diagnostics.contracts.json'))['contract_dependencies']
decls = sidecar['declarations']
report = json.load(open(run + '/resource-diagnostics.json'))

rows_of = collections.defaultdict(list)  # function -> [(file, status, diagnostics)]
unit_evidence = collections.defaultdict(list)  # name -> [(file, kind, detail)]
for unit in report['units']:
    for function in unit['functions']:
        rows_of[function['function']].append((unit['file'], function['status'], function['diagnostics']))
    evidence = unit.get('contract_body_evidence') or {}
    for name, why in (evidence.get('failed') or {}).items():
        unit_evidence[name].append((unit['file'], 'failed', why))
    for name in evidence.get('unvalidated') or []:
        unit_evidence[name].append((unit['file'], 'unvalidated', None))
    for name, why in (evidence.get('validated') or {}).items():
        unit_evidence[name].append((unit['file'], 'validated', why))


def is_libc(name):
    return 'provider-libc' in decls[name].get('module', '')


def reason(name):
    entry = decls[name]
    bodies = entry.get('bodies')
    if any(p.get('origin') == 'declared' for p in entry.get('premises') or []):
        return 'named-premise'
    if entry['body_standing'] == 'partly-validated':
        return 'uncovered-clause-kind'
    if bodies is None:
        return 'no-body'
    if 'conflict' in bodies.get('failed_by', []):
        return 'conflict'
    if bodies.get('failed_units'):
        return 'failed:' + ','.join(bodies['failed_by'])
    rows = rows_of.get(name)
    if rows:
        statuses = sorted({status for _, status, _ in rows})
        return 'row-' + '/'.join(statuses)
    return 'scope-body-no-summary'


ranking = []
for name, entry in decls.items():
    if entry['body_standing'] not in ('declared-assumption', 'partly-validated'):
        continue
    if is_libc(name):
        continue
    ranking.append((entry['safe_rows'], name, entry['body_standing'], reason(name),
                    entry.get('module', '').split('contracts/')[-1]))
ranking.sort(key=lambda row: (-row[0], row[1]))
sizes = collections.Counter()
safe_by_reason = collections.Counter()
for safe, name, standing, why, module in ranking:
    key = why.split(':')[0]
    sizes[key] += 1
    safe_by_reason[key] += safe
    if safe or show_all:
        print(f'{safe:3d} {name:45s} {standing:20s} {why:28s} {module}')
print('declarations by reason:', dict(sizes))
print('Safe-row mentions by reason:', dict(safe_by_reason))
print('counts', sidecar['counts'])
rollup = sidecar['safe_rollup']
print('rollup', {k: v for k, v in rollup.items() if not isinstance(v, (list, dict))})
