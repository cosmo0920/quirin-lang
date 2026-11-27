#!/usr/bin/env python3
"""Compare complete frozen production reports; retain invocation differences."""
import argparse
import filecmp
import hashlib
import json
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--before', required=True, type=Path)
parser.add_argument('--after', required=True, type=Path)
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def differences(left, right, path=''):
    if type(left) is not type(right):
        return [{'path': path, 'before': left, 'after': right}]
    if isinstance(left, dict):
        result = []
        for key in sorted(left.keys() | right.keys()):
            child = path + '/' + key.replace('~', '~0').replace('/', '~1')
            if key not in left or key not in right:
                result.append({'path': child, 'before_present': key in left,
                               'after_present': key in right})
            else:
                result.extend(differences(left[key], right[key], child))
        return result
    if isinstance(left, list) and len(left) == len(right):
        return [difference for index, (a, b) in enumerate(zip(left, right))
                for difference in differences(a, b, path + '/' + str(index))]
    return [] if left == right else [{'path': path, 'before': left, 'after': right}]


records = []
for suffix in ['.json', '.contracts.json', '.chunk-contracts.json', '.source-effects.json']:
    name = 'resource-diagnostics' + suffix
    left, right = args.before / name, args.after / name
    records.append({'file': name, 'before_bytes': left.stat().st_size,
                    'after_bytes': right.stat().st_size,
                    'before_sha256': digest(left), 'after_sha256': digest(right),
                    'byte_equal': filecmp.cmp(left, right, shallow=False)})
name = 'resource-diagnostics.invocation.json'
left, right = args.before / name, args.after / name
invocation = {'before_sha256': digest(left), 'after_sha256': digest(right),
              'differences': differences(json.loads(left.read_text()),
                                         json.loads(right.read_text()))}
report = json.loads((args.after / 'resource-diagnostics.json').read_text())
units = report['units']
receipt = {'before': str(args.before.resolve()), 'after': str(args.after.resolve()),
           'comparisons': records, 'invocation': invocation,
           'units': len(units), 'unique_files': len({unit['file'] for unit in units}),
           'functions': sum(len(unit.get('functions', [])) for unit in units),
           'scope': 'Complete report and evidence sidecars, no removed or normalized fields. Invocation differences are recorded in full for separate review; their presence is not evidence parity.'}
args.output.write_text(json.dumps(receipt, indent=2) + '\n')
print(json.dumps(receipt, indent=2))
raise SystemExit(0 if all(record['byte_equal'] for record in records) else 1)
