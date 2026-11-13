#!/usr/bin/env python3
"""Compare two runs' function statuses (custody report) and null rows, with
every embedded analyzer revision spelled `REV`.

usage: statuses.py BASE_DIR TIP_DIR BASE_REV TIP_REV
Exit 0 when every function row's status is identical and the reports are
byte-identical once the two revisions are spelled alike; prints the counts.
"""
import collections
import json
import sys
from pathlib import Path


def rows(directory):
    report = json.loads((directory / 'resource-diagnostics.json').read_text())
    out = {}
    for unit in report['units']:
        for function in unit.get('functions', []):
            key = (unit['library'], unit['file'], unit.get('row', ''), function['function'])
            out.setdefault(key, []).append(function['status'])
    return out


def spelled(directory, name, revs):
    text = (directory / name).read_text()
    for rev in revs:
        text = text.replace(rev, 'REV')
    return text


def main():
    base, tip = Path(sys.argv[1]), Path(sys.argv[2])
    revs = [r for r in sys.argv[3:5]]
    revs = sorted(set(revs + [r[:9] for r in revs]), key=len, reverse=True)
    a, b = rows(base), rows(tip)
    moved = [(k, a.get(k), b.get(k)) for k in sorted(set(a) | set(b)) if a.get(k) != b.get(k)]
    tally = collections.Counter(s for v in b.values() for s in v)
    print(json.dumps({'functions': sum(len(v) for v in b.values()), 'statuses': dict(sorted(tally.items())),
                      'moved': len(moved)}))
    for entry in moved[:20]:
        print('moved', entry)
    same = True
    for name in ['resource-diagnostics.json', 'resource-diagnostics.components.json',
                 'resource-diagnostics.contracts.json', 'resource-diagnostics.chunk-contracts.json',
                 'resource-diagnostics.source-effects.json', 'system-null.jsonl', 'system-null.summary.json',
                 'system-outer.json']:
        equal = spelled(base, name, revs) == spelled(tip, name, revs)
        print(name, 'identical-modulo-revision' if equal else 'DIFFERS')
        same &= equal
    sys.exit(0 if same and not moved else 1)


if __name__ == '__main__':
    main()
