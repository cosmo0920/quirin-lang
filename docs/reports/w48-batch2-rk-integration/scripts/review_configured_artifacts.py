#!/usr/bin/env python3
"""Preserve duplicate configurations and classify all configured report fields."""
import copy
import json
from pathlib import Path
import sys

repo, measured, output = map(Path, sys.argv[1:4])
base = Path('/tmp/qcore-w48-bottom-up-batch2/private/w48-bottom-up-batch2')

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

result = {}
for route in ('system', 'slice'):
    before = base / f'tip-{route}-final'
    after = measured / route
    old = json.loads((before / 'resource-diagnostics.json').read_text())
    new = json.loads((after / 'resource-diagnostics.json').read_text())
    ca = json.loads((before / 'resource-diagnostics.contracts.json').read_text())
    cb = json.loads((after / 'resource-diagnostics.contracts.json').read_text())
    assert normalize(ca) == normalize(cb), f'{route}: additional contract change'
    for name in ('resource-diagnostics.chunk-contracts.json', 'resource-diagnostics.source-effects.json'):
        assert (before / name).read_bytes() == (after / name).read_bytes(), (route, name)
    assert len(old['units']) == len(new['units'])
    adjusted = copy.deepcopy(old)
    configurations, review_paths = [], []
    for index, (a, b) in enumerate(zip(old['units'], new['units'])):
        ac, bc = a['compile_configuration'], b['compile_configuration']
        identities = {
            ac['dependencies']['resource_contract_sources']: bc['dependencies']['resource_contract_sources'],
            ac['inventory_sha256']: bc['inventory_sha256'],
        }
        assert normalize(ac, identities) == normalize(bc), (route, index)
        if ac != bc:
            configurations.append({'unit_index': index, 'file': a['file'], 'identities': identities})
        adjusted['units'][index]['compile_configuration'] = bc
        assert len(a['functions']) == len(b['functions'])
        for x, y in zip(a['functions'], b['functions']):
            if x != y:
                fields = {k for k in x.keys() | y.keys() if x.get(k) != y.get(k)}
                assert fields == {'review_holds'}, (route, index, x.get('function'), fields)
                assert normalize(x) == normalize(y)
                review_paths.append({'unit_index': index, 'file': a['file'], 'function': x['function']})
    assert normalize(adjusted) == normalize(new), f'{route}: unexplained report field'
    result[route] = {
        'units': len(old['units']), 'configuration_rows': sum(len(u['functions']) for u in new['units']),
        'compile_configuration_provenance_only': configurations,
        'review_hold_checkout_prefix_only': review_paths,
        'all_other_report_fields_identical': True,
        'contract_sidecar_identical_after_checkout_prefix_only': True,
        'raw_contract_keys': {'before': sorted(ca['contracts']), 'after': sorted(cb['contracts'])},
        'chunk_and_source_effect_artifacts_byte_identical': True,
        'monkey_source_artifact': 'not emitted by current route',
    }
output.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({r: {k:v for k,v in d.items() if k in ('units','configuration_rows','all_other_report_fields_identical','chunk_and_source_effect_artifacts_byte_identical')} for r,d in result.items()}, indent=2))
