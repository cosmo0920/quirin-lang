#!/usr/bin/env python3
"""Classify the complete corpus diff without changing any measured output."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

repo, before, after, diff_path, destination = map(Path, sys.argv[1:6])
spec = importlib.util.spec_from_file_location('canonical_diff', repo / 'docs/reports/corpus-repin/diff_artifacts.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
raw = json.loads(diff_path.read_text())
a = json.loads((before / 'resource-diagnostics.contracts.json').read_text())
b = json.loads((after / 'resource-diagnostics.contracts.json').read_text())
left, lc = module.contract_sources(a)
right, rc = module.contract_sources(b)
assert not lc and not rc and left.keys() == right.keys()
assert all(left[k]['text'] == right[k]['text'] for k in left)

def normalize(value, identities=None):
    identities = identities or {}
    if isinstance(value, str):
        value = value.replace('/tmp/qcore-w48-bottom-up-batch2', '$CHECKOUT').replace(str(repo), '$CHECKOUT')
        return identities.get(value, value)
    if isinstance(value, list):
        return [normalize(item, identities) for item in value]
    if isinstance(value, dict):
        result = {normalize(k, identities): normalize(v, identities) for k, v in value.items()}
        assert len(result) == len(value), 'normalization collision'
        return result
    return value

homes_a = {h['function']: h for h in a['cross_unit_link_resolution']['imported']}
homes_b = {h['function']: h for h in b['cross_unit_link_resolution']['imported']}
assert homes_a.keys() == homes_b.keys() and len(homes_a) == 6
home_ids = {homes_a[k]['home_environment_sha256']: homes_b[k]['home_environment_sha256'] for k in homes_a}
assert normalize(a, home_ids) == normalize(b), 'additional contract sidecar change'
units = raw['artifacts']['diagnostics']['units']
assert not units['added'] and not units['removed']
for row in units['changed']:
    assert set(row['fields']) == {'cross_unit_link_resolution'}
    values = row['fields']['cross_unit_link_resolution']
    assert normalize(values['before'], home_ids) == normalize(values['after'])
effects = raw['artifacts']['source-effects']['results']
assert not effects['added'] and not effects['removed']
reviewed = []
for row in effects['changed']:
    fields = row['fields']
    assert set(fields) == {'bounds', 'evidence', 'import_identity', 'task', 'witness'}
    old, new = fields['task']['before'], fields['task']['after']
    identities = {
        old['unit']['build_digest']: new['unit']['build_digest'],
        fields['import_identity']['before']['configuration_digest']: fields['import_identity']['after']['configuration_digest'],
        fields['witness']['before']['task_digest']: fields['witness']['after']['task_digest'],
    }
    assert len(old['obligations']) == len(new['obligations'])
    identities.update({x['obligation_id']: y['obligation_id'] for x, y in zip(old['obligations'], new['obligations'])})
    for values in fields.values():
        assert normalize(values['before'], identities) == normalize(values['after']), row['key']
    reviewed.append({'key': row['key'], 'changed_fields': sorted(fields), 'identity_pairs': identities,
                     'classification': 'checkout/configuration and derived task/evidence identities only'})
for name in ('source', 'chunk-contracts'):
    for entry in raw['artifacts'][name].values():
        if isinstance(entry, dict) and 'counts' in entry:
            assert all(entry['counts'][k] == 0 for k in ('added', 'removed', 'changed'))
    assert not raw['artifacts'][name]['envelope']
summary = {
    'meaning': 'All raw fields retained in the canonical diff; comparison-only normalization never modifies checker output',
    'canonical_diff_sha256': hashlib.sha256(diff_path.read_bytes()).hexdigest(),
    'raw_contract_source_keys': {'before': sorted(a['sources']), 'after': sorted(b['sources'])},
    'raw_key_counts': {'before': len(a['sources']), 'after': len(b['sources']),
                       'added': len(b['sources'].keys() - a['sources'].keys()),
                       'removed': len(a['sources'].keys() - b['sources'].keys())},
    'normalized_modules': {'before': len(left), 'after': len(right), 'changed_texts': 0,
                           'collisions_before': lc, 'collisions_after': rc},
    'diagnostic_units': units['counts'],
    'changed_diagnostic_fields': ['cross_unit_link_resolution.home_environment_sha256'],
    'cross_unit_home_environment_identities': home_ids,
    'contract_sidecar_changed_keys': sorted(raw['artifacts']['contracts']['keys']),
    'source_effect_rows_reviewed': reviewed,
}
destination.write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps({k: summary[k] for k in ('raw_key_counts', 'normalized_modules', 'diagnostic_units', 'changed_diagnostic_fields')}, indent=2))
