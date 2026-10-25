#!/usr/bin/env python3
"""Byte identity of two measure.py route directories (AUDIT-CANONICAL-PATHS).

Usage: compare.py ROUTE LEFT_DIR RIGHT_DIR OUT_JSON [--sub FROM=TO ...]

Every artifact a route publishes is compared byte for byte: the report and
each sidecar, the null and outer projections, and the console and audit logs.
Not compared: the drivers' own records (`invocation.json`, `corpus/run.json`,
the corpus driver's `console.log`, which prints run.json), which carry
timestamps and output paths, and `*.exit` (checked equal separately). In the
logs, the `preparation:` line is dropped: it sums per-unit wall-clock time and
differs between any two runs (docs/agents/validation.md).

`--sub FROM=TO` rewrites the RIGHT side's text before comparing, longest FROM
first, through placeholder tokens so no replacement is rewritten again. The
substitution is reported exactly: per file and in total, how often each FROM
occurred on the right, and how often each TO already occurred on the right
before the rewrite (a collision would make the map ambiguous; it is reported,
never hidden). Files still different after the rewrite get a structural
summary: every differing JSON leaf path (list indices as `*`) with counts and
examples (digest-valued keys collapsed). Each side's driver record is
summarised too: the audit's exit and, for a driver exit 101, the snapshot
files that changed. Read-only; exit 0 when every compared file is identical
after the substitution and both audits exited 0, 1 otherwise.
"""
import argparse
import collections
import json
import pathlib
import re
import sys

SKIP = {'invocation.json', 'corpus/run.json'}
TIMING = re.compile(rb'^preparation: .*\n', re.MULTILINE)


def artifacts(root, route):
    files = set()
    for path in root.rglob('*'):
        if not path.is_file():
            continue
        relative = str(path.relative_to(root))
        if relative in SKIP or relative.endswith('.exit'):
            continue
        if route == 'corpus' and relative == 'console.log':
            continue
        files.add(relative)
    return files


def normalise(relative, data):
    if relative.endswith('.log'):
        data = TIMING.sub(b'', data)
    return data


def substitute(data, subs):
    counts = {}
    collisions = {}
    for index, (source, target) in enumerate(subs):
        collisions[target] = data.count(target.encode())
    for index, (source, target) in enumerate(subs):
        token = f'\0SUB{index}\0'.encode()
        counts[source] = data.count(source.encode())
        data = data.replace(source.encode(), token)
    for index, (source, target) in enumerate(subs):
        data = data.replace(f'\0SUB{index}\0'.encode(), target.encode())
    return data, counts, collisions


def leaves(value, path, out):
    if isinstance(value, (dict, list)) and not value:
        out[path] = json.dumps(value)
    elif isinstance(value, dict):
        for key, item in value.items():
            leaves(item, path + (key,), out)
    elif isinstance(value, list):
        for index, item in enumerate(value):
            leaves(item, path + (index,), out)
    else:
        out[path] = value


HEX64 = re.compile(r'[0-9a-f]{64}')
HEX24 = re.compile(r'\b[0-9a-f]{24}\b')


def pattern(path):
    """List indices become `*`; digest-valued keys become `<sha256>` (or
    `<id24>`), so a digest-keyed map is one pattern, not one per key."""
    parts = []
    for part in path:
        if isinstance(part, int):
            parts.append('*')
        else:
            part = HEX64.sub('<sha256>', part)
            parts.append(HEX24.sub('<id24>', part))
    return '/'.join(parts)


def structural(left, right):
    def documents(data):
        text = data.decode()
        try:
            return [json.loads(text)]
        except json.JSONDecodeError:
            return [json.loads(line) for line in text.splitlines() if line.strip()]
    left_docs, right_docs = documents(left), documents(right)
    if len(left_docs) != len(right_docs):
        return {'documents': [len(left_docs), len(right_docs)]}
    patterns = collections.Counter()
    examples = {}
    for number, (one, two) in enumerate(zip(left_docs, right_docs)):
        a, b = {}, {}
        leaves(one, (number,), a)
        leaves(two, (number,), b)
        for path in sorted(set(a) | set(b), key=str):
            if a.get(path, '<absent>') != b.get(path, '<absent>'):
                key = pattern(path[1:])
                patterns[key] += 1
                examples.setdefault(key, [str(a.get(path, '<absent>'))[:240],
                                          str(b.get(path, '<absent>'))[:240]])
    return {'differing_leaves': sum(patterns.values()), 'pattern_count': len(patterns),
            'patterns': [{'path': key, 'count': count, 'example': examples[key]}
                         for key, count in patterns.most_common(300)]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('route')
    parser.add_argument('left', type=pathlib.Path)
    parser.add_argument('right', type=pathlib.Path)
    parser.add_argument('out', type=pathlib.Path)
    parser.add_argument('--sub', action='append', default=[])
    args = parser.parse_args()
    subs = sorted((tuple(item.split('=', 1)) for item in args.sub), key=lambda pair: -len(pair[0]))
    left_files, right_files = artifacts(args.left, args.route), artifacts(args.right, args.route)
    exits = {side: (root / f'{args.route}.exit').read_text().strip()
             for side, root in (('left', args.left), ('right', args.right))}
    # The driver's own record: the audit's exit and, for a driver exit 101,
    # exactly which input changed during the route.
    drivers = {}
    for side, root in (('left', args.left), ('right', args.right)):
        record = json.loads((root / 'invocation.json').read_text())
        changed = sorted(key for key in set(record['inputs_before']) | set(record['inputs_after'])
                         if record['inputs_before'].get(key) != record['inputs_after'].get(key))
        drivers[side] = {'exit': record.get('exit'), 'audit_exit': record.get('audit_exit', record.get('exit')),
                         'identity_error': record.get('identity_error'), 'snapshot_files_changed': changed,
                         'externals_stable': record['externals_before'] == record['externals_after'],
                         'binary_stable': record['binary_sha256'] == record['binary_sha256_after'],
                         'binary_sha256': record['binary_sha256']}
    result = {'route': args.route, 'left': str(args.left), 'right': str(args.right),
              'substitution': [{'from': source, 'to': target} for source, target in subs],
              'exits': exits, 'drivers': drivers, 'only_left': sorted(left_files - right_files),
              'only_right': sorted(right_files - left_files), 'files': {},
              'substitution_totals': collections.Counter(),
              'target_collisions_total': collections.Counter()}
    identical = (not result['only_left'] and not result['only_right']
                 and drivers['left']['audit_exit'] == drivers['right']['audit_exit'] == 0)
    for relative in sorted(left_files & right_files):
        left = normalise(relative, (args.left / relative).read_bytes())
        right_raw = normalise(relative, (args.right / relative).read_bytes())
        right, counts, collisions = substitute(right_raw, subs)
        entry = {'bytes': [len(left), len(right_raw)], 'raw_identical': left == right_raw,
                 'identical_after_substitution': left == right}
        if subs:
            entry['substituted'] = {key: value for key, value in counts.items() if value}
            entry['target_collisions'] = {key: value for key, value in collisions.items() if value}
            result['substitution_totals'].update(counts)
            result['target_collisions_total'].update(collisions)
        if left != right:
            identical = False
            if relative.endswith(('.json', '.jsonl')):
                entry['structural'] = structural(left, right)
        result['files'][relative] = entry
    result['identical'] = identical
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=1) + '\n')
    print(json.dumps({'route': args.route, 'identical': identical,
                      'files': len(result['files']),
                      'different': sorted(k for k, v in result['files'].items()
                                          if not v['identical_after_substitution'])}, indent=1))
    return 0 if identical else 1


if __name__ == '__main__':
    raise SystemExit(main())
