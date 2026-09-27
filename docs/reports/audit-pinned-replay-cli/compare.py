#!/usr/bin/env python3
"""Exhaustive W46 archive versus migrated pinned-run comparison.

The only permitted raw artifact difference is the checkout-root prefix on
contract-source keys. This reports that difference; it does not rewrite either
artifact. All other bytes, including verdict-bearing rows, must match.
"""
import argparse
import collections
import gzip
import hashlib
import json
from pathlib import Path
from source_effect_provenance import compare_source_effects

ARTIFACTS = (
    'resource-diagnostics.json',
    'resource-diagnostics.chunk-contracts.json',
    'resource-diagnostics.contracts.json',
    'resource-diagnostics.source-effects.json',
    'monkey-source.json',
)
ARCHIVE_ROOT = '/home/cosmo/Documents/GitHub/qcore-c'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def module_key(raw):
    # The raw key is recorded separately. The normalized key only pairs modules.
    from posixpath import normpath
    path = normpath(raw)
    marker = '/contracts/'
    if marker not in path:
        # Package-provided modules use a stable provider-qualified key.
        return path
    return path.rsplit(marker, 1)[1]


def inventory(sources):
    result = {}
    for raw, text in sources.items():
        key = module_key(raw)
        if key in result:
            raise ValueError(f'normalized contract key collision: {key}')
        result[key] = (raw, text)
    return result


def compare_contracts(old_bytes, new_bytes, new_root):
    old = json.loads(old_bytes)
    new = json.loads(new_bytes)
    old_sources = inventory(old.pop('sources'))
    new_sources = inventory(new.pop('sources'))
    names = set(old_sources) | set(new_sources)
    added = sorted(set(new_sources) - set(old_sources))
    removed = sorted(set(old_sources) - set(new_sources))
    changed_text = []
    unexpected_raw_keys = []
    expected_raw_key_changes = []
    for name in sorted(set(old_sources) & set(new_sources)):
        old_raw, old_text = old_sources[name]
        new_raw, new_text = new_sources[name]
        if old_text != new_text:
            changed_text.append(name)
        expected = (old_raw.replace(ARCHIVE_ROOT, new_root, 1)
                    if old_raw.startswith(ARCHIVE_ROOT) else old_raw)
        if new_raw != expected:
            unexpected_raw_keys.append({'module': name, 'before': old_raw, 'after': new_raw})
        elif new_raw != old_raw:
            expected_raw_key_changes.append(name)
    rewritten = old_bytes.replace(ARCHIVE_ROOT.encode(), new_root.encode())
    return {
        'raw_byte_identical': old_bytes == new_bytes,
        'expected_root_rewrite_byte_identical': rewritten == new_bytes,
        'archive_sha256': digest(old_bytes),
        'migrated_sha256': digest(new_bytes),
        'metadata_equal': old == new,
        'metadata_changed_fields': sorted(k for k in set(old) | set(new) if old.get(k) != new.get(k)),
        'module_count_before': len(old_sources),
        'module_count_after': len(new_sources),
        'added_modules': added,
        'removed_modules': removed,
        'changed_module_texts': changed_text,
        'expected_checkout_prefix_key_changes': len(expected_raw_key_changes),
        'unexpected_raw_key_changes': unexpected_raw_keys,
        'normalized_module_names_accounted': len(names),
        'passes': rewritten == new_bytes and not (added or removed or changed_text or unexpected_raw_keys),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('measured', type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    run = json.loads((args.measured / 'run.json').read_text())
    if run['exit'] != 0:
        parser.error(f'measured replay did not exit 0: {run["exit"]}')
    # The measured folder is private/audit-pinned-replay-cli/new/replay-jobs4;
    # parent index 3 reaches the checkout root.
    root = str(args.measured.resolve().parents[3])
    result = {'archive': str(args.archive), 'measured': str(args.measured),
              'measured_exit': run['exit'], 'artifacts': {}}
    archive_run = json.loads(gzip.decompress((args.archive / 'run.json.gz').read_bytes()))
    passed = True
    for name in ARTIFACTS:
        before = gzip.decompress((args.archive / (name + '.gz')).read_bytes())
        after = (args.measured / name).read_bytes()
        if name == 'resource-diagnostics.contracts.json':
            entry = compare_contracts(before, after, root)
        elif name == 'resource-diagnostics.source-effects.json':
            entry = compare_source_effects(before, after, ARCHIVE_ROOT, root,
                                           str(Path(archive_run['command'][2]) / 'build/compile_commands.json'))
        else:
            entry = {'archive_sha256': digest(before), 'migrated_sha256': digest(after),
                     'byte_identical': before == after, 'passes': before == after}
            if name == 'resource-diagnostics.json':
                left, right = json.loads(before), json.loads(after)
                entry['unit_count_before'] = len(left['units'])
                entry['unit_count_after'] = len(right['units'])
                entry['unit_verdicts_before'] = dict(sorted(collections.Counter(x['status'] for x in left['units']).items()))
                entry['unit_verdicts_after'] = dict(sorted(collections.Counter(x['status'] for x in right['units']).items()))
        result['artifacts'][name] = entry
        passed &= entry['passes']
    old_path = str(Path(archive_run['command'][-1]).parent)
    new_path = str(Path(run['command'][-1]).parent)
    old_log = gzip.decompress((args.archive / 'audit.log.gz').read_bytes())
    new_log = (args.measured / 'audit.log').read_bytes()
    normalized_old_log = old_log.replace(old_path.encode(), b'<REPORT>')
    normalized_new_log = new_log.replace(new_path.encode(), b'<REPORT>')
    result['audit_log'] = {'archive_sha256': digest(old_log), 'migrated_sha256': digest(new_log),
                           'raw_byte_identical': old_log == new_log,
                           'identical_after_report_path_only': normalized_old_log == normalized_new_log,
                           'line_count_before': len(old_log.splitlines()),
                           'line_count_after': len(new_log.splitlines())}
    passed &= normalized_old_log == normalized_new_log
    result['passes'] = bool(passed)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
