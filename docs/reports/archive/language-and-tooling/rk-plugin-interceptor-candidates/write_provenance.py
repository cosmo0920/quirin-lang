#!/usr/bin/env python3
"""Record exact fresh measurement identities, input profiles and artifact hashes."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

repo = Path(__file__).resolve().parents[3]
root, output = map(Path, sys.argv[1:3])
base = Path('/home/cosmo/Documents/GitHub/qcore-c/private/w48-w44-rk-integration/measure-combined')
head = (root / 'head').read_text().strip()
gated = 'd899486b47a582707d015a070c9e83e82a99a5a4'
paths = ['Cargo.toml', 'Cargo.lock', 'crates', 'compiler', 'providers', 'contract', 'scripts', 'proof', 'third_party']
changes = subprocess.check_output(['git', '-C', str(repo), 'diff', '--name-only', gated, head, '--', *paths], text=True).splitlines()
assert not changes

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

routes = {}
for route in ('corpus', 'system', 'slice'):
    assert (root / f'{route}.exit').read_text().strip() == '0'
    report = json.loads((root / route / 'resource-diagnostics.json').read_text())
    contracts = json.loads((root / route / 'resource-diagnostics.contracts.json').read_text())
    outer = json.loads((root / f'{route}-outer.json').read_text())
    nulls = [json.loads(line) for line in (root / f'{route}-null.jsonl').read_text().splitlines()]
    files = [root / route / name for name in ('resource-diagnostics.json', 'resource-diagnostics.contracts.json', 'resource-diagnostics.source-effects.json', 'resource-diagnostics.chunk-contracts.json')]
    if route == 'corpus':
        files.append(root / route / 'monkey-source.json')
    files.extend([root / f'{route}-null.jsonl', root / f'{route}-outer.json'])
    routes[route] = {
        'exit': 0, 'source_revision': report['fluent_bit_revision'],
        'resource_schema_version': report['schema_version'], 'resource_scope': report['scope'],
        'source_profile': contracts['source_profile'] if route == 'corpus' else contracts['profile'],
        'profile_pin_role': 'pinned corpus' if route == 'corpus' else contracts['profile_pin_role'],
        'selection': None if route == 'corpus' else {key: contracts['unit_selection'][key] for key in ('name', 'sha256', 'scope')},
        'null_model_versions': sorted({row['identity']['model_version'] for row in nulls}),
        'outer_rule': outer['rule'], 'outer_schema_version': outer['schema_version'],
        'artifacts': {str(path.relative_to(root)): {'sha256': sha(path),
                        'baseline_sha256': sha(base / path.relative_to(root)),
                        'bytes_identical_to_baseline': path.read_bytes() == (base / path.relative_to(root)).read_bytes()}
                      for path in files},
    }
result = {
    'measured_repository_source': head, 'gated_implementation_source': gated,
    'implementation_paths_changed_since_gate': changes, 'checked_paths': paths,
    'measurement_path': str(root.resolve()),
    'binary_sha256': (root / 'binary.sha256').read_text().split()[0],
    'cross_unit_map_sha256': (root / 'link-map.sha256').read_text().split()[0],
    'routes': routes,
    'assumptions': [
        'declared resource/bounds contracts and selected source profiles remain trusted premises',
        'W44 linked-program resolution, compatible types/calling conventions and interposition premise',
        'outer statements and neutral callback/context-lifetime restrictions require human deployment review',
        'candidate inventory is runtime-only and establishes no static list invocation closure',
    ],
    'bounds': 'the selected generic custody/null models and configured selections; no general bounds, object-liveness or C-body safety claim',
}
output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
print('recorded', head, {route: row['source_revision'] for route, row in routes.items()})
