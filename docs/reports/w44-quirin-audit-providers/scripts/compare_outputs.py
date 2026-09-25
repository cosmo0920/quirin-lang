#!/usr/bin/env python3
"""Compare one configured-route run's outputs with a baseline run's.

Usage: compare_outputs.py BASE RUN [--added FILE:KEY]... [--log-mask-lines N,...]
                          [--log-inserted N,...]

Every baseline artifact (`resource-diagnostics.json`, its `.contracts.json`,
`.chunk-contracts.json` and `.source-effects.json` siblings,
`monkey-source.json`) must exist in RUN. An artifact is `identical` when its
bytes are equal. Otherwise it is `identical-after-removing` when deleting the
declared top-level keys (`--added FILE:KEY`, which must be absent from the
baseline and present in RUN) leaves a JSON value equal to the baseline's
(every remaining key, leaf and array order); anything else is `different`, with
the first differing leaves listed. `audit.log` is compared line by line with
the `preparation:` line masked (its three wall-clock figures differ between any
two runs; W44-AUDIT-PARALLEL-UNITS) and with any `--log-mask-lines` (1-based)
masked too; `--log-inserted` names RUN lines (1-based) that the baseline does
not have, which are printed and removed before the comparison. Every masked or
inserted line is printed. Files present only in RUN are
listed. Exit 0 only when nothing is `different`.
"""
import json
import sys
from pathlib import Path

ARTIFACTS = [
    'resource-diagnostics.json',
    'resource-diagnostics.contracts.json',
    'resource-diagnostics.chunk-contracts.json',
    'resource-diagnostics.source-effects.json',
    'monkey-source.json',
]


def leaves(value, path=''):
    if isinstance(value, dict):
        for key in sorted(value):
            yield from leaves(value[key], f'{path}/{key}')
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from leaves(item, f'{path}[{index}]')
    else:
        yield path, value


def main():
    arguments = sys.argv[1:]
    if len(arguments) < 2:
        raise SystemExit(__doc__)
    base, run = Path(arguments[0]), Path(arguments[1])
    added = {}
    masked_lines = set()
    inserted = set()
    rest = arguments[2:]
    while rest:
        flag = rest.pop(0)
        if flag == '--added':
            name, key = rest.pop(0).split(':', 1)
            added.setdefault(name, []).append(key)
        elif flag == '--log-inserted':
            inserted.update(int(n) for n in rest.pop(0).split(','))
        elif flag == '--log-mask-lines':
            masked_lines.update(int(n) for n in rest.pop(0).split(','))
        else:
            raise SystemExit(f'unknown argument {flag}')
    results = {}
    ok = True
    for name in ARTIFACTS:
        left, right = base / name, run / name
        if not right.exists():
            results[name] = 'missing'
            ok = False
            continue
        a, b = left.read_bytes(), right.read_bytes()
        if a == b:
            if name in added:
                results[name] = 'different: a declared added key is absent'
                ok = False
            else:
                results[name] = 'identical'
            continue
        ja, jb = json.loads(a), json.loads(b)
        keys = added.get(name, [])
        if keys and all(k not in ja and k in jb for k in keys):
            for key in keys:
                del jb[key]
            if ja == jb:
                results[name] = f'identical-after-removing {keys}'
                continue
        la, lb = dict(leaves(ja)), dict(leaves(jb))
        differing = sorted(p for p in set(la) | set(lb) if la.get(p, '<absent>') != lb.get(p, '<absent>'))
        results[name] = {'different': len(differing), 'first': differing[:20]}
        ok = False
    log_a = (base / 'audit.log').read_text().splitlines()
    log_b_all = (run / 'audit.log').read_text().splitlines()
    inserted_lines = [{'run_line': n, 'text': log_b_all[n - 1]} for n in sorted(inserted)]
    log_b = [line for n, line in enumerate(log_b_all, start=1) if n not in inserted]
    masked = []
    log_ok = len(log_a) == len(log_b)
    for number, (x, y) in enumerate(zip(log_a, log_b), start=1):
        if x == y:
            continue
        if x.startswith('preparation:') and y.startswith('preparation:') or number in masked_lines:
            masked.append({'line': number, 'base': x, 'run': y})
            continue
        log_ok = False
        masked.append({'line': number, 'base': x, 'run': y, 'unmasked': True})
    results['audit.log'] = {'identical_after_masking': log_ok, 'lines': len(log_b_all),
                            'inserted_lines': inserted_lines, 'differing_lines': masked}
    ok = ok and log_ok
    extra = sorted(p.name for p in run.iterdir() if not (base / p.name).exists())
    results['only_in_run'] = extra
    print(json.dumps({'base': str(base), 'run': str(run), 'results': results, 'ok': ok}, indent=2))
    raise SystemExit(0 if ok else 1)


if __name__ == '__main__':
    main()
