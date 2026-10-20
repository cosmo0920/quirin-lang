#!/usr/bin/env python3
"""Check configured context identities, retaining semantic and usage differences."""
from pathlib import Path
import gzip
import json
import sys

REPO = Path(__file__).resolve().parents[4]
REPORT = Path(__file__).resolve().parents[1]
OUT = REPORT / 'artifacts' / sys.argv[1]


def load(side, family):
    return json.loads(gzip.decompress((OUT / side / (family + '.json.gz')).read_bytes()))


def normalize(value, mapping=None):
    if isinstance(value, str):
        for name in ['inputs-base', 'inputs-tip']:
            value = value.replace(str(REPO / 'private/w48-bottom-up-batch3' / name), '$INPUTS')
        value = value.replace(str(REPO), '$ANALYZER')
        return (mapping or {}).get(value, value)
    if isinstance(value, list):
        return [normalize(v, mapping) for v in value]
    if isinstance(value, dict):
        result = {normalize(k, mapping): normalize(v, mapping) for k, v in value.items()}
        assert len(result) == len(value), 'normalization collision'
        return result
    return value


def pair(mapping, old, new):
    if old != new:
        assert isinstance(old, str) and isinstance(new, str)
        assert old not in mapping or mapping[old] == new
        mapping[old] = new


result = dict(route=sys.argv[1], meaning='Finite context substitutions and separately retained declaration usage counts. Semantic diagnostic changes are reported, never classified as identity.')
a, b = [normalize(load(side, 'diagnostics')) for side in ['before', 'after']]
assert len(a['units']) == len(b['units'])
units = []
for i, (x, y) in enumerate(zip(a['units'], b['units'])):
    mapping = {}
    xx, yy = x['compile_configuration'], y['compile_configuration']
    pair(mapping, xx['dependencies']['resource_contract_sources'], yy['dependencies']['resource_contract_sources'])
    pair(mapping, xx['inventory_sha256'], yy['inventory_sha256'])
    assert normalize(xx, mapping) == yy, ('unexplained compile configuration change', i)
    same = normalize(x, mapping) == y
    units.append(dict(index=i, library=x['library'], file=x['file'], exact_mapping=mapping,
                      compile_configuration_complete_equal=True,
                      complete_unit_payload_equal_after_mapping=same))
a['units'] = [normalize(x, unit['exact_mapping']) for x, unit in zip(a['units'], units)]
result['diagnostics_units'] = units
result['diagnostics_complete_payload_equal_after_mapping'] = a == b
result['non_identity_unit_indices'] = [x['index'] for x in units if not x['complete_unit_payload_equal_after_mapping']]
result['non_identity_evidence'] = ['row-comparison.json', 'full-row-differences.json.gz', 'precise-field-differences.json.gz']

mapping = {}
a, b = [normalize(load(side, 'contracts')) for side in ['before', 'after']]
x, y = [d['contract_dependencies']['declarations']['__errno_location'] for d in [a, b]]
pair(mapping, x['contract_sha256'], y['contract_sha256'])
pair(mapping, x['premises'][0]['statement'], y['premises'][0]['statement'])
assert a['contracts'].keys() == b['contracts'].keys()
changed = [k for k in a['contracts'] if a['contracts'][k] != b['contracts'][k]]
assert len(changed) == 1 and changed[0].endswith('/libc/errno.qv')
pair(mapping, a['contracts'][changed[0]], b['contracts'][changed[0]])
locks = [d['contract_revision']['locks'] for d in [a, b]]
assert locks[0].keys() == locks[1].keys()
for key in locks[0]:
    if locks[0][key] != locks[1][key]:
        assert key.endswith('/quirin-provider-libc/contracts.lock')
        pair(mapping, locks[0][key]['sha256'], locks[1][key]['sha256'])
contexts = []
for path in [('callback_members', 'lifetime_protocols', 'evidence', 'summaries'),
             ('callback_members', 'prepass', 'lifetime_protocols', 'summaries')]:
    x, y = a, b
    for key in path:
        x, y = x[key], y[key]
    removed, added = sorted(x.keys() - y.keys()), sorted(y.keys() - x.keys())
    assert len(removed) == len(added)
    if removed:
        assert len(removed) == 1 and x[removed[0]] == y[added[0]] == {}
        pair(mapping, removed[0], added[0])
        contexts.append(dict(path=path, before=removed[0], after=added[0], payload={}))
usage = []
x, y = [d['contract_dependencies']['declarations'] for d in [a, b]]
assert x.keys() == y.keys()
for function in x:
    for field in ['rows', 'safe_rows']:
        if x[function][field] != y[function][field]:
            usage.append(dict(function=function, field=field, before=x[function][field], after=y[function][field]))
            x[function][field] = y[function][field]
assert normalize(a, mapping) == b, 'unexplained contract artifact change'
result.update(contracts_exact_mapping=mapping, empty_callback_contexts=contexts,
              declaration_usage_adjustments=usage,
              contracts_complete_payload_equal_after_explicit_mapping_and_usage_adjustments=True)
result['other_three_families'] = {}
for family in ['source', 'chunk-contracts', 'source-effects']:
    paths = [OUT / side / (family + '.json.gz') for side in ['before', 'after']]
    assert paths[0].exists() == paths[1].exists(), family
    if paths[0].exists():
        assert normalize(load('before', family)) == normalize(load('after', family)), family
        result['other_three_families'][family] = dict(present=True, complete_payload_equal_after_checkout_normalization=True)
    else:
        result['other_three_families'][family] = dict(present=False, both_absent=True, meaning='Configured route does not emit this family; absence is not safety evidence.')
(OUT / 'context-identity-review.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(dict(route=sys.argv[1], units=len(units), semantic_unit_changes=result['non_identity_unit_indices'], usage_adjustments=usage)))
