#!/usr/bin/env python3
"""Derive configured identities from complete retained inputs; keep semantic deltas."""
from pathlib import Path
from collections import Counter
import argparse, copy, gzip, hashlib, json, re

repo = Path(__file__).resolve().parents[4]
report = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('route', choices=['system', 'slice'])
parser.add_argument('--artifacts-root', type=Path)
args = parser.parse_args()
route = args.route
out = (args.artifacts_root or report / 'artifacts') / route
load = lambda side, family: json.loads(gzip.decompress((out / side / (family + '.json.gz')).read_bytes()))
ser = lambda value, ordered=False: json.dumps(value, ensure_ascii=False, separators=(',', ':'), sort_keys=not ordered).encode()
sha = lambda value: hashlib.sha256(value).hexdigest()
roots = [str(repo / 'private/w48-bottom-up-batch4' / name) for name in ['inputs-base-final', 'inputs-tip-final']]
def norm(value):
    if isinstance(value, str):
        for root in roots:
            value = value.replace(root, '$INPUTS')
        return value
    if isinstance(value, list):
        return [norm(v) for v in value]
    if isinstance(value, dict):
        x = {norm(k): norm(v) for k, v in value.items()}
        assert len(x) == len(value), 'normalization collision'
        return x
    return value

def task_bytes(task):
    keys = ['schema_version', 'compiler', 'declared_driver', 'directory', 'file', 'arguments',
            'dialect', 'function_scope', 'source_input', 'preprocessor_options', 'include_paths',
            'definitions', 'raw_source_sha256', 'preprocessed_source_sha256', 'preprocessing_error',
            'dependencies', 'dialect_differences_sha256']
    assert set(task) in [set(keys), set(keys[:-1])]
    x = {key: task[key] for key in keys if key in task}
    assert set(x['compiler']) == {'program', 'version', 'target'}
    x['compiler'] = {key: x['compiler'][key] for key in ['program', 'version', 'target']}
    if isinstance(x['source_input'], dict):
        assert set(x['source_input']) == {'DeclaredFile'}
        value = x['source_input']['DeclaredFile']
        assert set(value) == {'spelled', 'directory'}
        x['source_input'] = {'DeclaredFile': {key: value[key] for key in ['spelled', 'directory']}}
    else:
        assert x['source_input'] == 'AnonymousText'
    # Actual nested frontend-option payloads are retained unchanged; every
    # resulting full byte digest must match the emitted production context.
    x['dependencies'] = dict(sorted(x['dependencies'].items()))
    return ser(x, ordered=True)

D = {side: load(side, 'diagnostics') for side in ['before', 'after']}
C = {side: load(side, 'contracts') for side in ['before', 'after']}
assert norm(C['before']['contracts']) == norm(C['after']['contracts'])
assumptions = {side: sha(ser(C[side]['contracts'])) for side in D}
base_receipt = json.loads((report / 'base-binary.json').read_text())
inputs = json.loads((report / 'runtime-inputs-final.json').read_text())
heads = {'before': base_receipt['build_head'], 'after': inputs['tip_revision']}
versions = {side: 'contract-hypothesis-graph.v1:' + heads[side] for side in D}
for side, label in [('before', 'base'), ('after', 'tip')]:
    inv = json.loads((out / side / 'invocation.json').read_text())
    assert inv['binary_sha256'] == inputs[label + '_binary_sha256']
assert len(D['before']['units']) == len(D['after']['units'])
units = []; builds = {}; restored_cfg = {}
for index, (a, b) in enumerate(zip(D['before']['units'], D['after']['units'])):
    assert (a['library'], a['file']) == (b['library'], b['file'])
    configurations = {}
    digests = {}
    for side, unit in [('before', a), ('after', b)]:
        cfg = unit['compile_configuration']; task = cfg['compile_task_identity']
        assert task['dependencies'] == cfg['dependencies']
        assert cfg['dependencies']['resource_contract_sources'] == assumptions[side]
        body = {k: v for k, v in cfg.items() if k != 'inventory_sha256'}
        assert sha(ser(body)) == cfg['inventory_sha256'], ('inventory digest', side, index)
        digest = sha(task_bytes(task)); digests[side] = digest
        for row in unit['functions']:
            context = row['hypothesis_context']
            assert context['assumptions'] == assumptions[side]
            assert context['build_identity'] == digest
            assert context['evidence_version'] == versions[side]
        configurations[side] = cfg
    x, y = [copy.deepcopy(configurations[side]) for side in ['before', 'after']]
    y['dependencies']['resource_contract_sources'] = assumptions['before']
    y['compile_task_identity']['dependencies']['resource_contract_sources'] = assumptions['before']
    y['inventory_sha256'] = sha(ser({k: v for k, v in y.items() if k != 'inventory_sha256'}))
    assert norm(x) == norm(y), ('unattributed compile-configuration field', index)
    assert sha(task_bytes(y['compile_task_identity'])) == digests['before']
    negative = copy.deepcopy(configurations['after']['compile_task_identity'])
    negative['dependencies']['resource_contract_sources'] = '0' * 64
    assert sha(task_bytes(negative)) != digests['after']
    assert digests['after'] not in builds or builds[digests['after']] == digests['before']
    builds[digests['after']] = digests['before']; restored_cfg[index] = y
    units.append({'index': index, 'library': a['library'], 'file': a['file'],
                  'build_identity': digests,
                  'inventory_sha256': {side: configurations[side]['inventory_sha256'] for side in D},
                  'complete_configuration_equal_after_exact_source_map_relocation': True,
                  'wrong_dependency_negative_rejected': True})

context_keys = {'artifact', 'property', 'semantic_profile', 'scope', 'bounds', 'assumptions',
                'source_identity', 'build_identity', 'evidence_version'}
context_count = 0
def restore(value):
    global context_count
    if isinstance(value, list):
        return [restore(v) for v in value]
    if not isinstance(value, dict):
        return value
    if set(value) == context_keys:
        assert value['assumptions'] == assumptions['after']
        assert value['evidence_version'] == versions['after']
        assert value['build_identity'] in builds, 'context outside derived selected-unit inventory'
        context_count += 1
        return value | {'assumptions': assumptions['before'],
                        'build_identity': builds[value['build_identity']],
                        'evidence_version': versions['before']}
    return {k: restore(v) for k, v in value.items()}

a = norm(copy.deepcopy(D['before'])); b = norm(restore(D['after']))
for index in range(len(a['units'])):
    b['units'][index]['compile_configuration'] = norm(restored_cfg[index])
semantic = []; removed = Counter(); added = Counter()
carrier_changes = []; summary_changes = []; reason_changes = []
allowed_carriers = {'system': {'flb_input_chunk_get_projected_write_size', 'handle_dlq_if_available', 'handle_output_event', 'flb_output_new'}, 'slice': {'in_tail_init', 'flb_tail_file_remove'}}
def assign_field(left, right, name):
    if name in right:
        left[name] = copy.deepcopy(right[name])
    else:
        left.pop(name, None)
def validate_carriers(row):
    values = row.get('null_premises', [])
    for value in values:
        assert set(value) == {'line', 'name', 'origin', 'variable'}
        assert value['name'] == f"null-deref:{row['function']}:{value['line']}:{value['variable']}"
    assert Counter(row.get('contract_dependencies', {}).get('null_premises', [])) == Counter(value['name'] for value in values)
def reason(unit):
    ds = [d for row in unit['functions'] for d in row.get('diagnostics', [])]
    held = sum(d['category'] == 'unsupported' and re.fullmatch(r"call to `[^`]+` whose declared contract awaits human review \(awaiting-human-review\)", d['description']) is not None for d in ds)
    pins = sum(d['category'] == 'unsupported' and re.fullmatch(r"call to `[^`]+` whose pinned contract is not in force: the audited source does not match its lock \(contract-pin-mismatch\)", d['description']) is not None for d in ds)
    counts = Counter(d['category'] for d in ds)
    text = f"resource projection unresolved: {counts['unsupported']-held-pins} unsupported operation(s), {counts['unknown_alias']} unresolved alias(es), {counts['summary_conflict']} custody summary conflict(s)"
    if held: text += f", {held} call(s) held awaiting human review"
    if pins: text += f", {pins} call(s) held for a contract pin mismatch"
    return text
for side, doc in D.items():
    total = Counter()
    for unit in doc['units']:
        counts = Counter(d['category'] for row in unit['functions'] for d in row.get('diagnostics', []))
        assert counts == Counter(unit['diagnostic_counts'])
        total.update(counts)
    assert total == Counter(doc['diagnostic_counts'])
for index, (x, y) in enumerate(zip(a['units'], b['units'])):
    assert len(x['functions']) == len(y['functions'])
    for row_index, (left, right) in enumerate(zip(x['functions'], y['functions'])):
        assert left['function'] == right['function'] and left['status'] == right['status']
        if left.get('diagnostics') != right.get('diagnostics'):
            ca, cb = [Counter(json.dumps(v, sort_keys=True) for v in row.get('diagnostics', [])) for row in [left, right]]
            before_only, after_only = [[json.loads(v) for v in c.elements()] for c in [ca-cb, cb-ca]]
            removed.update((v['category'], v['description']) for v in before_only)
            added.update((v['category'], v['description']) for v in after_only)
            semantic.append({'unit_index': index, 'row_index': row_index, 'library': x['library'],
                             'file': x['file'], 'function': left['function'], 'status': left['status'],
                             'removed': before_only, 'added': after_only})
            left['diagnostics'] = right['diagnostics']
        if left.get('null_premises') != right.get('null_premises'):
            assert left['function'] in allowed_carriers[route]
            validate_carriers(left); validate_carriers(right)
            carrier_changes.append({'unit_index': index, 'row_index': row_index, 'file': x['file'], 'function': left['function'], 'before': left.get('null_premises'), 'after': right.get('null_premises'), 'before_dependencies': left.get('contract_dependencies', {}).get('null_premises'), 'after_dependencies': right.get('contract_dependencies', {}).get('null_premises')})
            assign_field(left, right, 'null_premises')
            assign_field(left['contract_dependencies'], right['contract_dependencies'], 'null_premises')
        if left.get('contract_dependencies', {}).get('summaries') != right.get('contract_dependencies', {}).get('summaries'):
            assert route == 'system' and x['file'] == 'src/flb_task.c' and left['function'] == 'flb_task_destroy'
            assert left.get('contract_dependencies', {}).get('summaries') is None
            assert right['contract_dependencies']['summaries'] == ['mk_list_entry_is_orphan']
            summary_changes.append({'unit_index': index, 'row_index': row_index, 'file': x['file'], 'function': left['function'], 'before': None, 'after': right['contract_dependencies']['summaries'], 'meaning': 'Caller consultation of source-derived header helper with the checked two-pointer NULL conjunction; repeated caller sessions are not independent native definitions.'})
            assign_field(left['contract_dependencies'], right['contract_dependencies'], 'summaries')
    if x.get('reason') != y.get('reason'):
        for side, unit in [('before', x), ('after', y)]:
            assert unit['reason'] == reason(D[side]['units'][index])
        reason_changes.append({'unit_index': index, 'file': x['file'], 'before': x['reason'], 'after': y['reason'], 'all_counts_recomputed_from_full_diagnostics': True})
        x['reason'] = y['reason']
    x['diagnostic_counts'] = y['diagnostic_counts']
a['diagnostic_counts'] = b['diagnostic_counts']
assert a == b, 'unattributed configured diagnostic/body/context field beyond retained diagnostic deltas'

c = norm(copy.deepcopy(C['before'])); d = norm(restore(C['after']))
time_receipt = json.loads((out / 'time-identity-review.json').read_text())
if 'before_key' in time_receipt:
    for path in time_receipt['paths']:
        target = d
        for key in path:
            target = target[key]
        assert target.pop(time_receipt['after_key']) == {}
        assert time_receipt['before_key'] not in target
        target[time_receipt['before_key']] = {}
set_orders = []
for function, declaration in c['contract_dependencies']['declarations'].items():
    if 'hypothesis_evidence' not in declaration:
        continue
    left = declaration['hypothesis_evidence']['contexts']
    right = d['contract_dependencies']['declarations'][function]['hypothesis_evidence']['contexts']
    for side in ['before', 'after']:
        raw = C[side]['contract_dependencies']['declarations'][function]['hypothesis_evidence']['contexts']
        texts = [ser(value) for value in raw]
        assert texts == sorted(set(texts)), ('producer BTreeSet<String> order/uniqueness', side, function)
    rebound = [ser(value) for value in right]
    assert len(rebound) == len(set(rebound)), 'identity reconstruction collapsed two contexts'
    permutation = sorted(range(len(right)), key=lambda i: rebound[i])
    ordered = [right[i] for i in permutation]
    assert left == ordered, ('complete rebound context set', function)
    if right != ordered:
        set_orders.append({'function': function, 'contexts': len(right), 'rebound_order_permutation': permutation, 'meaning': 'contract_dependencies.rs records BTreeSet<String> of complete context JSON; hash relocation changes its lexical order. Both original arrays are verified canonical and unique, and the complete reconstructed context set is unchanged.'})
    d['contract_dependencies']['declarations'][function]['hypothesis_evidence']['contexts'] = ordered
assert c == d, 'unattributed configured contract artifact field'
families = {}
for family in ['source', 'chunk-contracts', 'source-effects']:
    paths = [out / side / (family + '.json.gz') for side in ['before', 'after']]
    assert paths[0].exists() == paths[1].exists()
    if paths[0].exists():
        assert norm(load('before', family)) == norm(load('after', family)), family
    families[family] = {'present': paths[0].exists(), 'complete_payload_equal_after_explicit_root_normalization': paths[0].exists()}
result = {'route': route, 'meaning': 'Every context assumption/build/version digest is reconstructed from complete retained source maps, canonical production compile-task field ordering and actual binary build heads. Only exact input-root relocation and those derived identities are substituted in comparison. All original artifacts remain intact; every changed diagnostic stays explicit. No actual witness is transported between contexts.',
          'assumptions': assumptions, 'evidence_versions': versions, 'units': units,
          'contexts_reconstructed': context_count, 'context_set_order_changes': set_orders, 'semantic_rows': semantic, 'null_carrier_changes': carrier_changes, 'source_summary_consultation_changes': summary_changes, 'unit_reason_changes': reason_changes,
          'null_carrier_meaning': 'Conditional custody-read premise carriers move with represented source guards and newly evaluated reads. Their complete old/new payloads remain explicit; they supply no new independent null-property fact.',
          'removed_families': [{'category': k[0], 'description': k[1], 'count': v} for k, v in removed.items()],
          'added_families': [{'category': k[0], 'description': k[1], 'count': v} for k, v in added.items()],
          'custody_statuses_identical': True,
          'all_other_diagnostic_metadata_equal_after_exact_reconstruction_and_retained_semantic_changes': True,
          'complete_contract_artifact_equal_after_exact_context_and_time_reconstruction': True,
          'other_families': families,
          'raw_artifact_sha256': {side: {family: sha(gzip.decompress((out / side / (family + '.json.gz')).read_bytes())) for family in ['diagnostics', 'contracts']} for side in D},
          'source_algorithms_sha256': {name: sha((repo / name).read_bytes()) for name in ['crates/qcore-audit/src/compile_db.rs', 'crates/qcore-audit/src/current.rs', 'crates/qcore-audit/src/projection.rs', 'crates/qcore-c-gateway/src/clang.rs', 'crates/qcore-c-gateway/src/legacy_c.rs', 'crates/qcore-audit/src/contract_dependencies.rs']}}
(out / 'context-identity-review.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'route': route, 'units': len(units), 'contexts': context_count,
                  'diagnostic_rows': len(semantic), 'carrier_rows': len(carrier_changes), 'summary_consultations': len(summary_changes), 'all_other_fields_attributed': True}))
