#!/usr/bin/env python3
"""STATUS-COMPACT: re-derive every headline figure on docs/STATUS.md from its primary source.

Usage: rederive.py GATE_JSON [OUT.json]

GATE_JSON is the newest landing gate record (an `integration/gate.json`).
Nothing is read from docs/STATUS.md itself or from any older status prose.
"""
import json
import subprocess
import sys
from collections import Counter
from pathlib import Path

gate_path = Path(sys.argv[1])
out = {}

# 1. Live evaluation profiles and their published baselines.
profiles = {}
for evaluation in sorted(Path('contract/evaluations').glob('*/evaluation.json')):
    e = json.loads(evaluation.read_text())
    if e.get('policy', {}).get('kind') != 'live':
        continue
    b = json.loads(Path(e['baseline']['path']).read_text())
    f = b['figures']
    profiles[e['name']] = {
        'source_revision': e['source']['revision'],
        'measured_revision': b['measured_revision'],
        'baseline_source': b['source'],
        'units': f.get('units'), 'unit_statuses': f.get('unit_statuses'),
        'rows': f.get('rows'), 'row_statuses': f.get('row_statuses'),
        'null_rows': f.get('null_rows'), 'null_verdicts': f.get('null_verdicts'),
        'baseline_history_entries': len(e.get('baseline_history', [])),
    }
out['live_profiles'] = profiles
out['frozen_profiles'] = sorted(
    json.loads(p.read_text())['name'] for p in Path('contract/evaluations').glob('*/evaluation.json')
    if json.loads(p.read_text()).get('policy', {}).get('kind') != 'live')

# 2. The newest landing gate.
g = json.loads(gate_path.read_text())
out['gate'] = {'path': str(gate_path), **{k: g[k] for k in g if k != 'note'}}

# 3. Acceptance matrix.
m = json.loads(Path('docs/reviews/r0_3/acceptance_matrix.json').read_text())
out['matrix'] = {
    'named_followups': len(m['named_followups']),
    'full_conformance': m['header']['full_conformance'],
    'validation_note_head': m['validation']['note'][:120],
}
out['acceptance_full_conformance'] = json.loads(
    Path('docs/reviews/r0_3/acceptance.json').read_text()).get('full_conformance')

# 4. CT manifest.
ct = json.loads(Path('docs/reevaluation/ct_manifest.json').read_text())
cases = ct['cases']
mapped = [c for c in cases if c['mapping_status'] == 'mapped']
proof = [c['id'] for c in cases if c['stages'].get('proof', {}).get('status') == 'executed']
native = [c['id'] for c in cases if c['stages'].get('native_sanitized', {}).get('status') == 'executed']
ident = ct['artifact_identity']
out['ct'] = {'cases': len(cases), 'mapped': len(mapped), 'proof': proof,
             'distinct_source_fixture_digests': ident['distinct_source_fixture_digests'],
             'shared_fixture_groups': len(ident['shared_fixture_groups']),
             'native_sanitized': native,
             'stage_status_counts': {s: dict(Counter(c['stages'].get(s, {}).get('status') for c in cases))
                                     for s in ('proof', 'native_sanitized')}}

# 5. Lean.
out['lean'] = {
    'toolchain': Path('proof/lean-toolchain').read_text().strip(),
    'audit_modules': len(list(Path('proof').glob('Audit*.lean'))),
}

# 6. Contract corpus: the eleven inventory locks (what qcore_contract_corpus.rs pins)
#    and the user crates read through providers/user-crates-load-path.json.
locks = sorted(Path('providers').glob('*/contract-inventory.lock'))
modules = declarations = clauses = 0
per_lock = {}
for lock in locks:
    data = json.loads(lock.read_text())
    per_lock[lock.parent.name] = len(data['files'])
    for entry in data['files'].values():
        modules += 1
        declarations += entry.get('declaration_count', 0)
        clauses += entry.get('clause_count', 0)
tracked = subprocess.check_output(['git', 'ls-files', 'providers/*/contracts/**.qv']).decode().split()
by_crate = Counter(p.split('/')[1] for p in tracked)
locked_crates = {lock.parent.name for lock in locks}
out['contracts'] = {
    'inventory_locks': len(locks),
    'locked_modules': modules, 'locked_declarations': declarations, 'locked_clauses': clauses,
    'modules_per_lock': per_lock,
    'tracked_qv_modules': len(tracked),
    'user_crate_modules': {k: v for k, v in sorted(by_crate.items()) if k not in locked_crates},
}

text = json.dumps(out, indent=1, sort_keys=True)
if len(sys.argv) > 2:
    Path(sys.argv[2]).write_text(text + '\n')
print(text)
