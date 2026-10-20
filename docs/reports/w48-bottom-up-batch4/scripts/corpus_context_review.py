#!/usr/bin/env python3
"""Reconstruct corpus caller contexts and replay complete production rollups."""
from pathlib import Path
from collections import Counter, defaultdict
import copy, datetime, gzip, hashlib, json, subprocess

repo = Path(__file__).resolve().parents[4]
report = Path(__file__).resolve().parents[1]
private = repo / 'private/w48-bottom-up-batch4'
out = report / 'artifacts/corpus'
load = lambda side, family: json.loads(gzip.decompress((out / side / (family + '.json.gz')).read_bytes()))
ser = lambda value: json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
sha = lambda value: hashlib.sha256(value).hexdigest()
D = {side: load(side, 'diagnostics') for side in ['before', 'after']}
C = {side: load(side, 'contracts') for side in D}
roots = [str(private / name) for name in ['inputs-base-final', 'inputs-tip-final']]
def norm(value):
    if isinstance(value, str):
        for root in roots:
            value = value.replace(root, '$INPUTS')
        return value
    if isinstance(value, list):
        return [norm(v) for v in value]
    if isinstance(value, dict):
        result = {norm(k): norm(v) for k, v in value.items()}
        assert len(result) == len(value), 'normalization collision'
        return result
    return value
assert norm(C['before']['sources']) == norm(C['after']['sources'])
assumptions = {side: sha(ser(C[side]['sources'])) for side in D}
base = json.loads((report / 'base-binary.json').read_text())
inputs = json.loads((report / 'runtime-inputs-final.json').read_text())
heads = {'before': base['build_head'], 'after': inputs['tip_revision']}
versions = {side: 'contract-hypothesis-graph.v1:' + heads[side] for side in D}
context_keys = {'artifact', 'property', 'semantic_profile', 'scope', 'bounds', 'assumptions', 'source_identity', 'build_identity', 'evidence_version'}
frames = {}; units = []
assert len(D['before']['units']) == len(D['after']['units'])
for index, (x, y) in enumerate(zip(D['before']['units'], D['after']['units'])):
    assert (x['library'], x['file']) == (y['library'], y['file'])
    assert len(x['functions']) == len(y['functions'])
    for a, b in zip(x['functions'], y['functions']):
        assert a['function'] == b['function']
        left, right = a['hypothesis_context'], b['hypothesis_context']
        assert set(left) == set(right) == context_keys
        for side, context in [('before', left), ('after', right)]:
            assert context['assumptions'] == assumptions[side]
            assert context['evidence_version'] == versions[side]
        rb = right | {'assumptions': assumptions['before'], 'evidence_version': versions['before']}
        assert left == rb, ('non-relocation context change', x['file'], a['function'], [k for k in left if left[k] != rb[k]])
        key = ser({k: v for k, v in right.items() if k not in {'assumptions', 'evidence_version'}})
        assert key not in frames or frames[key] == left
        frames[key] = left
    sample = x['functions'][0]['hypothesis_context'] if x['functions'] else None
    units.append({'index': index, 'library': x['library'], 'file': x['file'], 'rows': len(x['functions']), 'build_identity': sample['build_identity'] if sample else None, 'source_identity': sample['source_identity'] if sample else None, 'complete_other_context_fields_identical': True})
context_count = 0
def restore(value):
    global context_count
    if isinstance(value, list): return [restore(v) for v in value]
    if not isinstance(value, dict): return value
    if set(value) == context_keys:
        assert value['assumptions'] == assumptions['after'] and value['evidence_version'] == versions['after']
        key = ser({k: v for k, v in value.items() if k not in {'assumptions', 'evidence_version'}})
        assert key in frames, 'context outside exact selected-row source/build inventory'
        context_count += 1
        return copy.deepcopy(frames[key])
    return {k: restore(v) for k, v in value.items()}
restore(C['after'])

# Every global context set and usage count is tied to exact raw caller rows.
usage_receipts = {}
for side in D:
    rows = Counter(); safe = Counter(); contexts = defaultdict(set)
    for unit in D[side]['units']:
        for row in unit['functions']:
            names = row.get('contract_dependencies', {}).get('declarations', [])
            assert len(names) == len(set(names))
            for name in names:
                rows[name] += 1; safe[name] += row['status'] == 'Safe'
                contexts[name].add(ser(row['hypothesis_context']))
    declarations = C[side]['contract_dependencies']['declarations']
    for name, declaration in declarations.items():
        assert declaration['rows'] == rows[name] and declaration['safe_rows'] == safe[name]
        if 'hypothesis_evidence' in declaration:
            evidence = declaration['hypothesis_evidence']
            assert [ser(c) for c in evidence['contexts']] == sorted(contexts[name])
            assert evidence['dependent_rows'] == rows[name] and evidence['dependent_safe_rows'] == safe[name]
    usage_receipts[side] = {'declarations': len(declarations), 'rows_and_safe_rows_recomputed': True, 'canonical_unique_context_sets_recomputed': True}

# Exact environments are produced with the existing production loader/pin tool.
environments = {}; environment_files = {}
for side, label in [('before', 'base'), ('after', 'tip')]:
    path = private / ('environment-' + label + '.json'); raw = path.read_bytes()
    environment = json.loads(raw)
    assert environment['source_assumptions'] == C[side]['sources']
    environments[side] = environment
    environment_files[side] = {'path': str(path), 'sha256': sha(raw)}
    (out / (side + '-environment.json.gz')).write_bytes(gzip.compress(raw, mtime=0))
    for record in C[side]['cross_unit_link_resolution']['imported']:
        assert record['home_environment_sha256'] == sha(raw)
assert norm(environments['before']) == norm(environments['after'])

helper = private / 'metadata-replay'; commands = []
for side, label in [('before', 'base'), ('after', 'tip')]:
    actual = private / ('corpus-' + side + '-final/corpus')
    for family, name in [('diagnostics', 'resource-diagnostics.json'), ('contracts', 'resource-diagnostics.contracts.json')]:
        assert sha((actual / name).read_bytes()) == sha(gzip.decompress((out / side / (family + '.json.gz')).read_bytes()))
    command = [str(helper), str(actual / 'resource-diagnostics.json'), str(actual / 'resource-diagnostics.contracts.json'), environment_files[side]['path'], str(out / (side + '-metadata-replay.json'))]
    log = private / ('corpus-' + side + '-metadata-replay-' + datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S%fZ') + '.log'); started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    with log.open('w') as stream: result = subprocess.run(command, stdout=stream, stderr=stream)
    commands.append({'command': command, 'exit': result.returncode, 'log': str(log), 'started': started, 'ended': datetime.datetime.now(datetime.timezone.utc).isoformat()})
    (out / 'metadata-replay-commands.json').write_text(json.dumps(commands, indent=2) + '\n')
    assert result.returncode == 0, str(log)

# Only caller consultations/counts may differ inside hypothesis evidence.
old = C['before']['contract_dependencies']['declarations']; new = C['after']['contract_dependencies']['declarations']
assert old.keys() == new.keys(), 'new declaration consultation needs separate authority review'
evidence_changes = []
for name in old:
    left, right = [record.get('hypothesis_evidence') for record in [old[name], new[name]]]
    assert (left is None) == (right is None)
    if left is None: continue
    immutable = lambda value: {k: v for k, v in value.items() if k not in {'contexts', 'dependent_rows', 'dependent_safe_rows'}}
    assert immutable(left) == immutable(right), (name, 'graph authority or disposition changed')
    rebound = restore(right)
    aa, bb = {ser(value) for value in left['contexts']}, {ser(value) for value in rebound['contexts']}
    evidence_changes.append({'function': name, 'before_rows': left['dependent_rows'], 'after_rows': right['dependent_rows'], 'before_safe_rows': left['dependent_safe_rows'], 'after_safe_rows': right['dependent_safe_rows'], 'added_contexts': [json.loads(value) for value in sorted(bb-aa)], 'removed_contexts': [json.loads(value) for value in sorted(aa-bb)], 'graph_disposition_warrant_standing_and_digest_identical': True})
receipt = {'route': 'corpus', 'meaning': 'Complete source/build/profile/scope/bounds contexts match after only source-map root relocation and the two actually built evidence-version stamps. No build/source digest is aliased. Global graph evidence remains open/declared, with every consultation context/count independently recomputed and the complete production dependency renderer freshly replayed. Environments differ only by exact input-root labels. No actual witness is transported between contexts.',
           'assumptions': assumptions, 'evidence_versions': versions, 'units': units, 'contexts_reconstructed': context_count,
           'usage_context_reconstruction': usage_receipts, 'hypothesis_evidence_consultation_changes': evidence_changes,
           'hypothesis_authority_immutable_fields_identical': True, 'environments': environment_files,
           'complete_environments_equal_after_exact_root_relocation': True,
           'metadata_replay_commands': commands, 'metadata_replay_binary_sha256': sha(helper.read_bytes()),
           'raw_artifact_sha256': {side: {family: sha(gzip.decompress((out / side / (family + '.json.gz')).read_bytes())) for family in ['diagnostics', 'contracts']} for side in D}}
(out / 'context-identity-review.json').write_text(json.dumps(receipt, indent=2) + '\n')
print(json.dumps({'route': 'corpus', 'units': len(units), 'contexts': context_count, 'authority_fields_identical': True, 'complete_production_metadata_replays': len(commands)}))
