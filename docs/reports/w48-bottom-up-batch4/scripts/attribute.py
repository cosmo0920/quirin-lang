#!/usr/bin/env python3
"""Retain every semantic leaf change, source coordinate and complete body-evidence delta."""
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import sys

repo = Path(__file__).resolve().parents[4]
report = Path(__file__).resolve().parents[1]
route = sys.argv[1]
out = report / 'artifacts' / route
source = Path('/home/cosmo/Documents/GitHub/qcore-c') / {
    'corpus': 'build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source',
    'system': 'private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source',
    'slice': 'private/w47-tail/attempt-1/prep/fluent-bit-clang-source',
}[route]
tracked = __import__('subprocess').check_output(['git', '-C', str(source), 'ls-files'], text=True).splitlines()
prefixes = {'chunkio': 'lib/chunkio/src/', 'monkey-libevent': 'lib/monkey/',
            'librdkafka': 'lib/librdkafka-2.15.0/src/', 'cmetrics': 'lib/cmetrics/src/',
            'ctraces': 'lib/ctraces/src/', 'fluent-bit': ''}
load = lambda side, family: json.loads(gzip.decompress((out / side / (family + '.json.gz')).read_bytes()))
a, b = [load(side, 'diagnostics') for side in ['before', 'after']]
raw = json.loads(gzip.decompress((out / 'full-row-differences.json.gz').read_bytes()))
classified = []
for pair in raw:
    old, new = pair['before'], pair['after']
    if old['status'] == new['status'] and old.get('diagnostics') == new.get('diagnostics'):
        continue
    before, after = [Counter(json.dumps(d, sort_keys=True) for d in x.get('diagnostics', [])) for x in [old, new]]
    removed, added = [[json.loads(d) for d in x.elements()] for x in [before-after, after-before]]
    observations = []
    for d in removed+added:
        origin = d.get('origin')
        if not origin or ':' not in origin:
            continue
        spelling, number = origin.rsplit(':', 1)
        if not number.isdigit():
            continue
        candidates = [p for p in tracked if p == spelling or p.endswith('/'+spelling)]
        preferred = [p for p in candidates if p.startswith(prefixes.get(pair['key'][0], ''))]
        candidates = preferred or candidates
        if len(candidates) != 1:
            observations.append(dict(origin=origin, unresolved_source_candidates=candidates))
            continue
        p = source / candidates[0]
        lines = p.read_text().splitlines()
        line = int(number)
        observations.append(dict(origin=origin, source_path=candidates[0], file_sha256=hashlib.sha256(p.read_bytes()).hexdigest(), excerpt='\n'.join(lines[max(0,line-3):line+3])))
    classified.append(dict(key=pair['key'], before=old['status'], after=new['status'], removed=removed, added=added, source_observations=observations))
# All body-evidence changes are retained. Every existing validated payload must
# remain; an added replay payload is checked against independently present base
# units, rather than counted as a newly validated distinct native definition.
known = {}
for unit in a['units']:
    e = unit.get('contract_body_evidence', {})
    for name, methods in e.get('validated', {}).items():
        value = dict(methods=methods, clauses=e.get('clauses', {}).get(name))
        known.setdefault(name, [])
        if value not in known[name]:
            known[name].append(value)
body_changes = []
for i, (left, right) in enumerate(zip(a['units'], b['units'])):
    x, y = [u.get('contract_body_evidence', {}) for u in [left, right]]
    if x == y:
        continue
    retained, additions = [], []
    for name, methods in x.get('validated', {}).items():
        assert y.get('validated', {}).get(name) == methods, (i, name, 'prior validated methods changed')
        assert y.get('clauses', {}).get(name) == x.get('clauses', {}).get(name), (i, name, 'prior clauses changed')
        retained.append(name)
    for name, methods in y.get('validated', {}).items():
        if name in x.get('validated', {}):
            continue
        payload = dict(methods=methods, clauses=y.get('clauses', {}).get(name))
        additions.append(dict(function=name, payload=payload, existed_in_baseline_units=payload in known.get(name, [])))
    body_changes.append(dict(index=i, library=left['library'], file=left['file'], before=x, after=y, prior_validated_payloads_retained=retained, new_replay_payloads=additions))
result = dict(route=route, selected_rows=sum(len(u['functions']) for u in a['units']), changed_status_or_diagnostic_rows=len(classified), safe_gains=[p['key'] for p in classified if p['before']!='Safe' and p['after']=='Safe'], safe_regressions=[p['key'] for p in classified if p['before']=='Safe' and p['after']!='Safe'], new_provisional_unsafe=[p['key'] for p in classified if p['before']!='Unsafe' and p['after']=='Unsafe'], rows=classified, body_evidence_changes=body_changes, meaning='All semantic changes retained for source review; source snippets do not by themselves discharge custody obligations. Header replay multiplicity is not distinct native proof coverage.')
(out/'attribution.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k not in ['rows','body_evidence_changes']},indent=2))
