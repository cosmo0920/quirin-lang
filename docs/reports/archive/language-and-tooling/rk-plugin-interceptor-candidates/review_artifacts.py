#!/usr/bin/env python3
"""Retain raw keys and classify the observed five-artifact provenance differences.

This is an evidence comparison only. It does not change any checker verdict.
"""
import copy
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

repo = Path(__file__).resolve().parents[3]
base, tip, canonical, destination = map(Path, sys.argv[1:5])
spec = importlib.util.spec_from_file_location('canonical_diff', repo / 'docs/reports/corpus-repin/diff_artifacts.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
raw = json.loads(canonical.read_text())
a = json.loads((base / 'resource-diagnostics.contracts.json').read_text())
b = json.loads((tip / 'resource-diagnostics.contracts.json').read_text())
left, lc = module.contract_sources(a)
right, rc = module.contract_sources(b)
assert not lc and not rc
assert left.keys() == right.keys()
assert all(left[key]['text'] == right[key]['text'] for key in left)
old_root = '/home/cosmo/Documents/GitHub/qcore-c'
new_root = '/tmp/qcore-rk-interceptor-candidates'

def normalize(value, replacements):
    if isinstance(value, str):
        value = value.replace(old_root, '$CHECKOUT').replace(new_root, '$CHECKOUT')
        return replacements.get(value, value)
    if isinstance(value, list):
        return [normalize(item, replacements) for item in value]
    if isinstance(value, dict):
        result = {normalize(key, replacements): normalize(item, replacements) for key, item in value.items()}
        assert len(result) == len(value), 'normalisation collision'
        return result
    return value

homes_a = a['cross_unit_link_resolution']['imported']
homes_b = b['cross_unit_link_resolution']['imported']
assert len(homes_a) == len(homes_b) == 5
home_ids = {old['home_environment_sha256']: new['home_environment_sha256'] for old, new in zip(homes_a, homes_b)}
assert normalize(a, home_ids) == normalize(b, {}), 'additional contract sidecar change'
units = raw['artifacts']['diagnostics']['units']
for row in units['changed']:
    assert set(row['fields']) == {'cross_unit_link_resolution'}
    values = row['fields']['cross_unit_link_resolution']
    assert normalize(values['before'], home_ids) == normalize(values['after'], {})
source_effects = raw['artifacts']['source-effects']['results']
reviewed = []
for row in source_effects['changed']:
    fields = row['fields']
    assert set(fields) == {'bounds', 'evidence', 'import_identity', 'task', 'witness'}
    old, new = fields['task']['before'], fields['task']['after']
    ids = {old['unit']['build_digest']: new['unit']['build_digest'],
           fields['import_identity']['before']['configuration_digest']: fields['import_identity']['after']['configuration_digest'],
           fields['witness']['before']['task_digest']: fields['witness']['after']['task_digest']}
    assert len(old['obligations']) == len(new['obligations'])
    ids.update({x['obligation_id']: y['obligation_id'] for x, y in zip(old['obligations'], new['obligations'])})
    for values in fields.values():
        assert normalize(values['before'], ids) == normalize(values['after'], {}), row['key']
    reviewed.append({'key': row['key'], 'changed_fields': sorted(fields),
                     'classification': 'cwd/configuration and derived task/evidence identities; all other fields equal'})
summary = {
    'meaning': 'raw verdicts compared before provenance normalization; no checker output modified',
    'canonical_diff_sha256': hashlib.sha256(canonical.read_bytes()).hexdigest(),
    'raw_contract_source_keys': {'before': sorted(a['sources']), 'after': sorted(b['sources'])},
    'raw_key_counts': {'before': len(a['sources']), 'after': len(b['sources']),
                       'added': len(b['sources'].keys() - a['sources'].keys()),
                       'removed': len(a['sources'].keys() - b['sources'].keys())},
    'normalized_modules': {'before': len(left), 'after': len(right), 'added': 0, 'removed': 0,
                           'changed_texts': 0, 'collisions_before': lc, 'collisions_after': rc},
    'diagnostic_units': {'before': units['counts']['before'], 'after': units['counts']['after'],
                         'changed_cross_unit_environment_identity_only': len(units['changed']),
                         'changed_function_statuses_or_diagnostics': 0},
    'cross_unit_home_environment_identities': home_ids,
    'contract_sidecar_changed_keys': sorted(raw['artifacts']['contracts']['keys']),
    'source_effect_rows_reviewed': reviewed,
    'other_artifact_rows': {name: {key: entry['counts'] for key, entry in raw['artifacts'][name].items()
                                  if isinstance(entry, dict) and 'counts' in entry}
                            for name in ('chunk-contracts', 'source', 'source-effects')},
}
configured = {}
for route in ('system', 'slice'):
    old_report = json.loads((base.parent / route / 'resource-diagnostics.json').read_text())
    new_report = json.loads((tip.parent / route / 'resource-diagnostics.json').read_text())
    old_contracts = json.loads((base.parent / route / 'resource-diagnostics.contracts.json').read_text())
    new_contracts = json.loads((tip.parent / route / 'resource-diagnostics.contracts.json').read_text())
    assert normalize(old_contracts, {}) == normalize(new_contracts, {})
    adjusted = copy.deepcopy(old_report)
    assert len(old_report['units']) == len(new_report['units'])
    configurations = []
    review_paths = []
    for index, (old_unit, new_unit) in enumerate(zip(old_report['units'], new_report['units'])):
        old_config, new_config = old_unit['compile_configuration'], new_unit['compile_configuration']
        ids = {old_config['dependencies']['resource_contract_sources']: new_config['dependencies']['resource_contract_sources'],
               old_config['inventory_sha256']: new_config['inventory_sha256']}
        assert normalize(old_config, ids) == normalize(new_config, {}), (route, index)
        if old_config != new_config:
            configurations.append({'unit_index': index, 'file': old_unit['file'], 'identity_changes': ids})
        adjusted['units'][index]['compile_configuration'] = new_config
        assert len(old_unit['functions']) == len(new_unit['functions'])
        for old_function, new_function in zip(old_unit['functions'], new_unit['functions']):
            if old_function != new_function:
                fields = {key for key in old_function.keys() | new_function.keys() if old_function.get(key) != new_function.get(key)}
                assert fields == {'review_holds'}
                assert normalize(old_function, {}) == normalize(new_function, {})
                review_paths.append({'unit_index': index, 'file': old_unit['file'], 'function': old_function['function']})
    assert normalize(adjusted, {}) == normalize(new_report, {})
    configured[route] = {'resource_diagnostic_units': len(old_report['units']),
                         'compile_configurations_changed_identity_only': configurations,
                         'review_hold_module_prefix_only': review_paths,
                         'all_other_resource_diagnostic_data_identical': True,
                         'full_contract_sidecar_identical_after_checkout_prefix_only': True,
                         'raw_contract_keys': {'before': sorted(old_contracts['contracts']), 'after': sorted(new_contracts['contracts'])}}
summary['configured_routes'] = configured
destination.write_text(json.dumps(summary, indent=2, sort_keys=True) + '\n')
archive = destination.with_name('artifact-diff.json.gz')
archive.write_bytes(gzip.compress(canonical.read_bytes(), mtime=0))
print(json.dumps({key: value for key, value in summary.items() if key not in ('raw_contract_source_keys', 'configured_routes')}, indent=2))
