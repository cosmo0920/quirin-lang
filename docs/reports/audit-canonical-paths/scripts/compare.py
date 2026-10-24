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
examples. Read-only; exit 0 when every compared file is identical after the
substitution, 1 otherwise.
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
    if isinstance(value, dict):
        for key, item in value.items():
            leaves(item, path + (key,), out)
    elif isinstance(value, list):
        for index, item in enumerate(value):
            leaves(item, path + (index,), out)
    else:
        out[path] = value


def pattern(path):
    return '/'.join('*' if isinstance(part, int) else part for part in path)


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
    return {'differing_leaves': sum(patterns.values()),
            'patterns': [{'path': key, 'count': count, 'example': examples[key]}
                         for key, count in patterns.most_common()]}


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
    result = {'route': args.route, 'left': str(args.left), 'right': str(args.right),
              'substitution': [{'from': source, 'to': target} for source, target in subs],
              'exits': exits, 'only_left': sorted(left_files - right_files),
              'only_right': sorted(right_files - left_files), 'files': {},
              'substitution_totals': collections.Counter(),
              'target_collisions_total': collections.Counter()}
    identical = not result['only_left'] and not result['only_right'] and exits['left'] == exits['right'] == '0'
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
