#!/usr/bin/env python3
"""Plan/apply unambiguous EXT-03 path-reference spellings in current consumers.

Only a unique path suffix with at least two components is rewritten. Ambiguous
basenames, historical report prose/logs, provider identity files, and source
bytes require separate reviewed treatment.
"""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import subprocess

MAPPING = Path('docs/reports/extension-compatibility/mapping.tsv')
REPORT = Path('docs/reports/extension-physical-migration/runtime-abi-reference-rewrites.json')
TEXT_EXT = {'.rs', '.json', '.md', '.yml', '.yaml', '.py', '.sh', '.toml', '.el'}


def current_consumer(path):
    if Path(path).suffix not in TEXT_EXT:
        return False
    if path.startswith(('compiler/tests/', 'crates/', 'examples/', 'editors/', 'scripts/', '.github/')):
        return True
    return path in {'README.md', 'docs/reevaluation/ct_manifest.json',
                    'docs/reviews/r0_3/acceptance_matrix.json'}


def mapping():
    with MAPPING.open(newline='') as stream:
        rows = list(csv.DictReader(stream, delimiter='\t'))
    old_paths = [row['old_path'] for row in rows]
    replacements = []
    for row in rows:
        new = row['proposed_new_path']
        if not new.endswith(('.qr', '.qrlib')):
            continue
        old_parts = row['old_path'].split('/')
        new_parts = new.split('/')
        assert len(old_parts) == len(new_parts)
        for length in range(2, len(old_parts) + 1):
            old_suffix = '/'.join(old_parts[-length:])
            if sum(path.endswith(old_suffix) for path in old_paths) == 1:
                replacements.append((old_suffix, '/'.join(new_parts[-length:])))
                break
    return sorted(replacements, key=lambda item: len(item[0]), reverse=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--apply', action='store_true')
    args = parser.parse_args()
    names = subprocess.check_output(['git', 'ls-files', '-z']).split(b'\0')
    replacements = mapping()
    changes = []
    for raw in names:
        if not raw:
            continue
        path = raw.decode()
        if not current_consumer(path):
            continue
        source = Path(path).read_bytes()
        try:
            before = source.decode('utf-8')
        except UnicodeDecodeError:
            continue
        after = before
        matches = []
        for old, new in replacements:
            count = after.count(old)
            if count:
                after = after.replace(old, new)
                matches.append({'old': old, 'new': new, 'occurrences': count})
        if after == before:
            continue
        encoded = after.encode('utf-8')
        changes.append({'path': path, 'before_sha256': hashlib.sha256(source).hexdigest(),
                        'after_sha256': hashlib.sha256(encoded).hexdigest(), 'replacements': matches})
        if args.apply:
            Path(path).write_bytes(encoded)
    result = {'mode': 'applied' if args.apply else 'dry-run',
              'changed_files': len(changes),
              'replacement_occurrences': sum(item['occurrences'] for change in changes for item in change['replacements']),
              'changes': changes,
              'boundary': 'Only unambiguous path suffixes with at least two components in current consumers. Remaining basename/virtual/historical references need review.'}
    if args.apply:
        REPORT.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({key: result[key] for key in ('mode', 'changed_files', 'replacement_occurrences')}))


if __name__ == '__main__':
    main()
