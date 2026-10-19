#!/usr/bin/env python3
"""Archive fresh receipts and compare legacy projections without hiding raw changes."""
import argparse
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
BASE_ROOT = ROOT / 'private/contract-hypothesis-graph-v1/baseline-source'
RESERVED = {'$hypothesis_graphs', '$hypothesis_graph_digests'}


def erase(value):
    if isinstance(value, list):
        return [erase(item) for item in value]
    if not isinstance(value, dict):
        return value
    removed = set(RESERVED)
    function_row = isinstance(value.get('function'), str) and ('status' in value or 'null' in value)
    declaration = 'contract_sha256' in value and 'body_standing' in value
    if function_row:
        removed.update(('hypothesis_context', 'hypothesis_graph', 'hypothesis_graph_sha256'))
    if declaration:
        removed.update(('hypothesis_graph', 'hypothesis_evidence', 'hypothesis_graph_sha256'))
    if isinstance(value.get('declarations'), list):
        removed.add('hypothesis_dependencies')
    return {key: erase(item) for key, item in value.items() if key not in removed}


def normalize(value):
    if isinstance(value, str):
        return value.replace(str(BASE_ROOT), '$ANALYZER').replace(str(ROOT), '$ANALYZER')
    if isinstance(value, list):
        return [normalize(item) for item in value]
    if isinstance(value, dict):
        pairs = [(normalize(key), normalize(item)) for key, item in value.items()]
        result = dict(pairs)
        assert len(result) == len(pairs), 'analyzer-root key normalization collision'
        return result
    return value


def differences(left, right, path=()):
    if isinstance(left, dict) and isinstance(right, dict):
        out = []
        for key in sorted(left.keys() | right.keys()):
            if key not in left or key not in right:
                out.append({'path': path + (key,), 'before_present': key in left,
                            'after_present': key in right, 'before': left.get(key), 'after': right.get(key)})
            else:
                out.extend(differences(left[key], right[key], path + (key,)))
        return out
    if isinstance(left, list) and isinstance(right, list) and len(left) == len(right):
        return [change for i, (a, b) in enumerate(zip(left, right))
                for change in differences(a, b, path + (i,))]
    return [] if left == right else [{'path': path, 'before': left, 'after': right}]


def write_gz(path, value):
    data = (json.dumps(value, indent=2) + '\n').encode()
    path.write_bytes(gzip.compress(data, mtime=0))


def load(path):
    return json.loads(path.read_text())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('base', type=Path)
    parser.add_argument('tip', type=Path)
    parser.add_argument('route', choices=['corpus', 'system', 'slice'])
    parser.add_argument('out', type=Path)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    names = {'diagnostics': 'resource-diagnostics.json', 'contracts': 'resource-diagnostics.contracts.json',
             'chunk-contracts': 'resource-diagnostics.chunk-contracts.json',
             'source-effects': 'resource-diagnostics.source-effects.json', 'source': 'monkey-source.json',
             'null': f'{args.route}-null.jsonl', 'null-summary': f'{args.route}-null.summary.json',
             'outer': f'{args.route}-outer.json'}
    data = {}
    for label, directory in [('before', args.base), ('after', args.tip)]:
        invocation = load(directory / 'invocation.json')
        assert invocation['exit'] == 0, f'{label} measurement failed'
        assert invocation['binary_sha256'] == invocation['binary_sha256_after'], 'binary changed'
        destination = args.out / label
        destination.mkdir()
        (destination / 'invocation.json').write_text(json.dumps(invocation, indent=2) + '\n')
        manifest = {}
        docs = {}
        for stem, name in names.items():
            path = directory / name
            if not path.exists():
                assert stem in {'source', 'chunk-contracts', 'source-effects'}, f'missing {path}'
                continue
            raw = path.read_bytes()
            manifest[name] = {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}
            (destination / (name + '.gz')).write_bytes(gzip.compress(raw, mtime=0))
            docs[stem] = [json.loads(line) for line in raw.splitlines()] if name.endswith('.jsonl') else json.loads(raw)
        (destination / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
        data[label] = docs
    a, b = data['before'], data['after']
    assert a.keys() == b.keys(), 'artifact inventory changed'
    raw = {key: differences(a[key], b[key]) for key in a}
    write_gz(args.out / 'raw-differences.json.gz', raw)
    left = normalize(erase(a))
    right = normalize(erase(b))
    projected = {key: differences(left[key], right[key]) for key in left}
    write_gz(args.out / 'erased-normalized-differences.json.gz', projected)
    units_a, units_b = left['diagnostics']['units'], right['diagnostics']['units']
    without_configuration = lambda units: [{k: v for k, v in u.items() if k != 'compile_configuration'} for u in units]
    # Compile-configuration identities remain in the raw and normalized diffs;
    # the invariant projection removes only that provenance record.
    checks = {
        'static_units_rows_full_diagnostics_reasons_dependencies_and_standing_equal': without_configuration(units_a) == without_configuration(units_b),
        'diagnostic_counts_equal': left['diagnostics']['diagnostic_counts'] == right['diagnostics']['diagnostic_counts'],
        'legacy_premises_equal': left['contracts']['contract_premises'] == right['contracts']['contract_premises'],
        'dependency_and_clause_standing_section_equal': left['contracts']['contract_dependencies'] == right['contracts']['contract_dependencies'],
        'contract_revision_equal': left['contracts']['contract_revision'] == right['contracts']['contract_revision'],
        'null_rows_except_identity_equal': [{k: v for k, v in r.items() if k != 'identity'} for r in left['null']] == [{k: v for k, v in r.items() if k != 'identity'} for r in right['null']],
        'null_summary_equal': left['null-summary'] == right['null-summary'],
        'runtime_projection_equal': left['outer'] == right['outer'],
    }
    counter = lambda units: dict(Counter(f['status'] for u in units for f in u['functions']))
    summary = {'route': args.route, 'checks': checks,
               'before': {'units': len(units_a), 'functions': counter(units_a), 'null_rows': len(left['null'])},
               'after': {'units': len(units_b), 'functions': counter(units_b), 'null_rows': len(right['null'])},
               'raw_differing_leaf_counts': {k: len(v) for k, v in raw.items()},
               'erased_normalized_differing_leaf_counts': {k: len(v) for k, v in projected.items()},
               'normalization': {'before_analyzer_root': str(BASE_ROOT), 'after_analyzer_root': str(ROOT), 'replacement': '$ANALYZER', 'key_collisions': 0},
               'provenance_review': 'all raw and erased differences retained; compile_configuration and null identity changes require separate attribution before acceptance',
               'all_invariant_checks_pass': all(checks.values())}
    (args.out / 'comparison.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary, indent=2))
    return 0 if all(checks.values()) else 1


if __name__ == '__main__':
    raise SystemExit(main())
