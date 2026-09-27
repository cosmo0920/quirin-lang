#!/usr/bin/env python3
"""Compare complete verdict-bearing artifacts, and retain measurement provenance."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('before', type=Path)
p.add_argument('after', type=Path)
p.add_argument('--output', type=Path, required=True)
p.add_argument('--archive', type=Path)
a = p.parse_args()
names = ['resource-diagnostics.json', 'resource-diagnostics.contracts.json', 'resource-diagnostics.chunk-contracts.json', 'resource-diagnostics.source-effects.json']

def run_record(path):
    run = json.loads((path / 'run.json').read_text())
    assert run['exit'] == 0, path
    metrics = dict(line.split('=', 1) for line in (path / 'time.txt').read_text().splitlines())
    assert metrics['exit'] == '0', path
    return {'run': run, 'metrics': {k: float(v) for k,v in metrics.items()}}

result = {'before': run_record(a.before), 'after': run_record(a.after), 'artifacts': [], 'scope': 'Exact bytes of all four verdict-bearing current-route artifacts; invocation metadata and measured console timing are not verdict parity inputs.'}
for name in names:
    left = (a.before / name).read_bytes()
    right = (a.after / name).read_bytes()
    result['artifacts'].append({'name': name, 'before_sha256': hashlib.sha256(left).hexdigest(), 'after_sha256': hashlib.sha256(right).hexdigest(), 'byte_identical': left == right})
    if a.archive:
        for label, data in [('before', left), ('after', right)]:
            directory = a.archive / label
            directory.mkdir(parents=True, exist_ok=True)
            (directory / (name+'.gz')).write_bytes(gzip.compress(data, mtime=0))
result['passed'] = all(x['byte_identical'] for x in result['artifacts'])
a.output.parent.mkdir(parents=True, exist_ok=True)
a.output.write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps({'passed': result['passed'], 'before': result['before']['metrics'], 'after': result['after']['metrics']}, indent=2))
raise SystemExit(0 if result['passed'] else 1)
