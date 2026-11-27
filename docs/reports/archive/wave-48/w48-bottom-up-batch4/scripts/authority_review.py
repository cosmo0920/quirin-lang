#!/usr/bin/env python3
"""Keep declaration authority and usage counts distinct in each fresh route."""
import gzip
import hashlib
import json
import subprocess
import datetime
from pathlib import Path
import sys

REPO = Path(__file__).resolve().parents[4]
REPORT = Path(__file__).resolve().parents[1]


def normalize(value):
    if isinstance(value, str):
        for name in ['inputs-base-final', 'inputs-tip-final', 'inputs-base', 'inputs-tip']:
            value = value.replace(str(REPO / 'private/w48-bottom-up-batch4' / name), '$INPUTS')
        return value
    if isinstance(value, list):
        return [normalize(x) for x in value]
    if isinstance(value, dict):
        result = {normalize(k): normalize(v) for k, v in value.items()}
        assert len(result) == len(value), 'normalization collision'
        return result
    return value


route = sys.argv[1]
out = REPORT / 'artifacts' / route
left, right = [json.loads(gzip.decompress((out / side / 'contracts.json.gz').read_bytes()))
               for side in ['before', 'after']]
prerequisites = []
context_proof = None
if route in ['system', 'slice', 'corpus']:
    private = REPO / 'private/w48-bottom-up-batch4'
    for script in ['time_identity.py', 'configured_context_review.py' if route != 'corpus' else 'corpus_context_review.py']:
        command = ['python3', str(REPORT / 'scripts' / script)] + ([] if script == 'corpus_context_review.py' else [route])
        log = private / (route + '-authority-' + datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S%fZ') + '-' + script + '.log')
        started = datetime.datetime.now(datetime.timezone.utc).isoformat()
        with log.open('w') as stream:
            result = subprocess.run(command, stdout=stream, stderr=stream)
        prerequisites.append({'command': command, 'exit': result.returncode, 'log': str(log), 'started': started, 'ended': datetime.datetime.now(datetime.timezone.utc).isoformat()})
        (out / 'authority-prerequisite-commands.json').write_text(json.dumps(prerequisites, indent=2) + '\n')
        assert result.returncode == 0, (script, str(log))
    context_proof = json.loads((out / 'context-identity-review.json').read_text())
    if route == 'corpus':
        assert context_proof['hypothesis_authority_immutable_fields_identical']
        assert all(record['exit'] == 0 for record in context_proof['metadata_replay_commands'])
    else:
        assert context_proof['complete_contract_artifact_equal_after_exact_context_and_time_reconstruction']
    for side in ['before', 'after']:
        raw = gzip.decompress((out / side / 'contracts.json.gz').read_bytes())
        assert hashlib.sha256(raw).hexdigest() == context_proof['raw_artifact_sha256'][side]['contracts']
a, b = [normalize(d['contract_dependencies']['declarations']) for d in [left, right]]
context_fields = []
if context_proof is not None:
    for name in a:
        if a[name].get('hypothesis_evidence') != b[name].get('hypothesis_evidence'):
            assert 'hypothesis_evidence' in a[name] and 'hypothesis_evidence' in b[name]
            context_fields.append({'function': name, 'artifact_path': ['contract_dependencies', 'declarations', name, 'hypothesis_evidence'], 'canonical_json_sha256': {side: hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest() for side, value in [('before', a[name]['hypothesis_evidence']), ('after', b[name]['hypothesis_evidence'])]}, 'context_counts': {'before': len(a[name]['hypothesis_evidence']['contexts']), 'after': len(b[name]['hypothesis_evidence']['contexts'])}, 'proof': 'context-identity-review.json: complete raw-artifact-bound reconstruction, including producer BTreeSet order and exact consulted-row counts; complete field payloads remain in both raw contracts archives'})
            b[name]['hypothesis_evidence'] = a[name]['hypothesis_evidence']
assert a.keys() == b.keys(), 'declaration inventory changed; inspect before extending this review'
native_promotions = []
if route == 'corpus':
    name = 'cmt_summary_quantile_get_value'
    expected = {'resource_borrowed(#0)', 'resource_borrowed(#1)', 'resource_return_none()'}
    assert a[name]['body_standing'] == 'declared-assumption' and b[name]['body_standing'] == 'validated'
    assert b[name]['clauses'] == {key: 'validated' for key in expected}
    old_body, new_body = a[name]['bodies'], b[name]['bodies']
    assert old_body == {'failed_by': [], 'failed_units': 0, 'unvalidated_units': 1, 'validated_by': [], 'validated_units': 0, 'withdrawn_units': 0}
    assert new_body == old_body | {'unvalidated_units': 0, 'validated_by': ['W4', 'entry'], 'validated_units': 1}
    diagnostic_docs = [json.loads(gzip.decompress((out / side / 'diagnostics.json.gz').read_bytes())) for side in ['before', 'after']]
    units = [next(u for u in doc['units'] if u['library'] == 'cmetrics' and u['file'] == 'cmt_summary.c') for doc in diagnostic_docs]
    assert name in units[0]['contract_body_evidence']['unvalidated']
    assert name not in units[0]['contract_body_evidence']['validated']
    assert name not in units[1]['contract_body_evidence']['unvalidated']
    assert units[1]['contract_body_evidence']['validated'][name] == ['W4', 'entry']
    assert units[1]['contract_body_evidence']['clauses'][name] == {key: ['W4', 'entry'] for key in expected}
    assert [next(f for f in u['functions'] if f['function'] == name)['status'] for u in units] == ['Unknown', 'Safe']
    assert a[name]['hypothesis_graph'] == b[name]['hypothesis_graph'] == {'atoms': {}, 'premises': {}, 'version': 1}
    native_promotions.append({'function': name, 'before': {key: a[name].get(key) for key in ['body_standing', 'bodies', 'clauses']}, 'after': {key: b[name].get(key) for key in ['body_standing', 'bodies', 'clauses']}, 'methods_and_all_three_clauses_match_complete_native_unit_evidence': True, 'complete_global_derivation_replayed': 'context-identity-review.json; before/after-metadata-replay.json', 'meaning': 'One existing declared contract gains checked native custody-body conformance, with no named graph premise. This is distinct from proving graph propositions or general C/null/bounds safety.'})
    b[name]['body_standing'] = a[name]['body_standing']
    b[name].pop('clauses')
changes = []
body_occurrences = []
for name in a:
    changed = [k for k in sorted(a[name].keys() | b[name].keys()) if a[name].get(k) != b[name].get(k)]
    authority = [k for k in changed if k not in ['rows', 'safe_rows', 'bodies']]
    if 'bodies' in changed:
        old, new = a[name]['bodies'], b[name]['bodies']
        assert old.keys() == new.keys()
        fields = [k for k in old if old[k] != new[k]]
        assert set(fields) <= {'validated_units', 'unvalidated_units'} or (route == 'corpus' and name == 'cmt_summary_quantile_get_value' and set(fields) == {'validated_units', 'unvalidated_units', 'validated_by'}), (name, fields)
        body_occurrences.append(dict(function=name, changed_fields=fields, before=old, after=new,
                                     meaning='Complete native/replay counts retained; the explicitly checked new cmetrics body conformance is in native_body_standing_promotions, while other caller-session repeats are not independent C definitions.'))
    assert not authority, (name, authority)
    if changed:
        changes.append(dict(function=name, changed_fields=changed, before=a[name], after=b[name]))
result = dict(route=route, declarations=len(a), changed=changes,
              inventory_equal=True, body_replay_occurrence_changes=body_occurrences, native_body_standing_promotions=native_promotions, unchanged_declared_modes_pins_and_graph_authority=True,
              hypothesis_context_rebindings=context_fields, prerequisite_commands=prerequisites,
              errno_remains_declared_assumption=True,
              before_counts=left['contract_dependencies']['counts'],
              after_counts=right['contract_dependencies']['counts'],
              before_safe_rollup=left['contract_dependencies']['safe_rollup'],
              after_safe_rollup=right['contract_dependencies']['safe_rollup'])
(out / 'authority-review.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(dict(route=route, declarations=len(a), changed=[x['function'] for x in changes])))
