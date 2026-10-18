#!/usr/bin/env python3
"""Record the prior primary runtime measurement's code/input equivalence to base.

This does not relabel the old observation as a fresh run. It records its actual
source SHA and verifies no implementation, contract, proof or audit-input change
occurred before this card's declared base.
"""
import hashlib
import json
from pathlib import Path
import subprocess

repo = Path(__file__).resolve().parents[3]
primary = Path('/home/cosmo/Documents/GitHub/qcore-c/private/w48-w44-rk-integration/measure-combined')
base = '1543259f0e32c8a003a9062e7de7f59a42dd4519'
measured = (primary / 'head').read_text().strip()
assert measured == 'ad526b5871c8cd05596c63d110932923431915d7'
paths = ['Cargo.toml', 'Cargo.lock', 'crates', 'compiler', 'providers', 'contract', 'scripts', 'proof', 'third_party']
changed = subprocess.check_output(['git', '-C', str(repo), 'diff', '--name-only', measured, base, '--', *paths], text=True).splitlines()
assert not changed, changed
map_path = repo / 'docs/reports/w44-cross-unit-summary/link-map.corpus.json'
link_hash = hashlib.sha256(map_path.read_bytes()).hexdigest()
assert link_hash == (primary / 'link-map.sha256').read_text().split()[0]
result = {'declared_base': base, 'primary_measured_source': measured,
          'observation': 'reused primary integration baseline, not a fresh run',
          'primary_path': str(primary), 'checked_paths': paths, 'changed_paths': changed,
          'primary_binary_sha256': (primary / 'binary.sha256').read_text().split()[0],
          'link_map_sha256': link_hash,
          'routes': ['corpus', 'system', 'slice'],
          'assumptions': 'same pinned/configured source inputs and W44 linked-program resolution/type/calling-convention/interposition premise; runtime envelopes remain conditional on review'}
print(json.dumps(result, indent=2))
