#!/usr/bin/env python3
"""EXT-03 tracked-input identity ledger; no parser or compiler claims."""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import subprocess

MAPPING = Path('docs/reports/extension-compatibility/mapping.tsv')
ROOT = Path('docs/reports/extension-physical-migration')


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def tracked():
    result = subprocess.run(['git', 'ls-files', '-z'], check=True, capture_output=True)
    return {name.decode() for name in result.stdout.split(b'\0') if name}


def selected():
    with MAPPING.open(newline='') as stream:
        rows = list(csv.DictReader(stream, delimiter='\t'))
    assets = [row for row in rows if row['proposed_new_path'].endswith(('.qr', '.qrlib'))]
    assert len(assets) == 426, len(assets)
    return assets


def snapshot():
    assets = selected()
    names = tracked()
    if any(row['old_path'] not in names for row in assets):
        raise SystemExit('pre-migration input is missing from the tracked index')
    record = {
        'base': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
        'scope': 'EXT-03: 421 runtime sources including 164 archived inputs, five ABI manifests',
        'assets': [
            {'old': row['old_path'], 'new': row['proposed_new_path'],
             'kind': row['parser_result'], 'sha256': digest(row['old_path'])}
            for row in assets
        ],
    }
    destination = ROOT / 'runtime-abi-baseline.json'
    destination.write_text(json.dumps(record, indent=2, sort_keys=True) + '\n')
    print(f'{destination}: {len(assets)} source/ABI assets')


def verify():
    record = json.loads((ROOT / 'runtime-abi-baseline.json').read_text())
    names = tracked()
    failures = []
    attribution = []
    for asset in record['assets']:
        old, new = asset['old'], asset['new']
        if old in names or new not in names:
            failures.append(f'tracked rename missing or alias retained: {old} -> {new}')
            continue
        current = digest(new)
        attribution.append({'old': old, 'new': new, 'kind': asset['kind'],
                            'before_sha256': asset['sha256'], 'after_sha256': current,
                            'byte_equal': asset['sha256'] == current})
        if asset['kind'] != 'abi_manifest' and current != asset['sha256']:
            failures.append(f'runtime source bytes changed: {old} -> {new}')
    report = {'base': record['base'], 'tip': subprocess.check_output(
        ['git', 'rev-parse', 'HEAD'], text=True).strip(), 'assets': attribution,
        'failures': failures,
        'note': 'ABI byte changes are attributed here but require reviewed reference diff and manifest parser/consumer parity.'}
    destination = ROOT / 'runtime-abi-attribution.json'
    destination.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print(f'{destination}: {len(attribution)} renamed, {len(failures)} failures')
    if failures:
        raise SystemExit('\n'.join(failures))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('mode', choices=('snapshot', 'verify'))
    args = parser.parse_args()
    {'snapshot': snapshot, 'verify': verify}[args.mode]()
