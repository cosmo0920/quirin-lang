#!/usr/bin/env python3
"""artifacts.py -- copy the card's measurement records into artifacts/: the
censuses and change lists (before2/after2 on main bfbf0d673, and the first
before/after pair on 958e76593), the probe rows for mk_vhost_set and
mk_vhost_set_property, and every report's sha256."""
import hashlib
import json
import pathlib
import shutil
import subprocess
import sys

here = pathlib.Path(__file__).resolve().parent
repo = here.parents[3]
private = repo / 'private/mvp'
slices = pathlib.Path('/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1')
out = here.parent / 'artifacts'
(out / 'census').mkdir(parents=True, exist_ok=True)
for name in sorted((private / 'census').glob('*.json')):
    shutil.copyfile(name, out / 'census' / name.name)

rows = {}
for run in ['probe-before', 'probe-after', 'probe-before2', 'probe-after2']:
    for side in ['tip', 'master']:
        report = json.load(open(private / run / f'{side}.json'))
        for unit in report['units']:
            for function in unit.get('functions', []):
                if function['function'] in ('mk_vhost_set', 'mk_vhost_set_property'):
                    rows.setdefault(run, {}).setdefault(side, {})[function['function']] = {
                        'status': function['status'],
                        'leaves': [f"{d.get('origin')} {d['category']}: {d['description']}"
                                   for d in function.get('diagnostics', [])],
                    }
(out / 'probe-rows.json').write_text(json.dumps(rows, indent=1, sort_keys=True) + '\n')

digests = {}
for label in ['before', 'after', 'before2', 'after2']:
    for route, path in [
        ('corpus', private / label / 'resource-diagnostics.json'),
        ('system', private / f'{label}-system' / 'resource-diagnostics.json'),
        ('slice', slices / f'mvp-{label}' / 'resource-diagnostics.json'),
    ]:
        if path.exists():
            digests[f'{label}/{route}'] = hashlib.sha256(path.read_bytes()).hexdigest()
for label in ['before', 'after', 'before2', 'after2']:
    binary = private / f'{label}.binary.sha256'
    if binary.exists():
        digests[f'{label}/binary'] = binary.read_text().split()[0]
for label in ['before2', 'after2']:
    digests[f'{label}/head'] = (private / f'{label}.head').read_text().strip()
(out / 'digests.json').write_text(json.dumps(digests, indent=1, sort_keys=True) + '\n')
print(json.dumps(digests, indent=1, sort_keys=True))
