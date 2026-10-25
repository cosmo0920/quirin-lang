#!/usr/bin/env python3
"""Verdict-level equality of two route reports (AUDIT-CANONICAL-PATHS).

Usage: verdicts.py REPORT_LEFT REPORT_RIGHT NULL_LEFT NULL_RIGHT

For the pair whose inputs are spelled differently (BASE through the mirror,
TIP on the original inputs) the reports cannot be byte-identical: compile
identities digest the spelled record, and header `__FILE__` literals change
length. This compares what a verdict is: every unit's status and reason code
and every row's status, keyed by (library, unit file, function, occurrence),
the status tallies, and every null-property row's verdict. Read-only; prints
JSON and exits 0 only when all are equal.
"""
import collections
import json
import sys


def rows(report):
    units, functions, seen = {}, {}, collections.Counter()
    for unit in report['units']:
        key = (unit['library'], unit['file'])
        units[key + (str(seen[key]),)] = (unit['status'], unit.get('reason_code'))
        seen[key] += 1
        local = collections.Counter()
        for row in unit.get('functions', []):
            name = key + (row['function'],)
            functions[name + (str(seen[key] - 1), str(local[name]))] = row['status']
            local[name] += 1
    return units, functions


def null_rows(path):
    out, seen = {}, collections.Counter()
    for line in open(path):
        if not line.strip():
            continue
        row = json.loads(line)
        key = json.dumps({k: row.get(k) for k in ('library', 'file', 'function')}, sort_keys=True)
        null = row.get('null') or {}
        out[key + str(seen[key])] = (row.get('custody'), row.get('combined'), null.get('verdict'),
                                     null.get('scope'), null.get('returns_non_null'),
                                     tuple(check.get('outcome') for check in null.get('checks', [])))
        seen[key] += 1
    return out


def main():
    left, right = (json.load(open(path)) for path in sys.argv[1:3])
    (lu, lf), (ru, rf) = rows(left), rows(right)
    ln, rn = null_rows(sys.argv[3]), null_rows(sys.argv[4])
    result = {
        'units': [len(lu), len(ru)], 'functions': [len(lf), len(rf)], 'null_rows': [len(ln), len(rn)],
        'unit_status_tally': [collections.Counter(s for s, _ in lu.values()),
                              collections.Counter(s for s, _ in ru.values())],
        'function_status_tally': [collections.Counter(lf.values()), collections.Counter(rf.values())],
        'null_verdict_tally': [collections.Counter(value[2] for value in ln.values()),
                               collections.Counter(value[2] for value in rn.values())],
        'custody_combined_tally': [collections.Counter(f'{value[0]}/{value[1]}' for value in ln.values()),
                                   collections.Counter(f'{value[0]}/{value[1]}' for value in rn.values())],
        'unit_differences': sorted(str(k) for k in set(lu) | set(ru) if lu.get(k) != ru.get(k))[:20],
        'function_differences': sorted(str(k) for k in set(lf) | set(rf) if lf.get(k) != rf.get(k))[:20],
        'null_differences': sorted(k for k in set(ln) | set(rn) if ln.get(k) != rn.get(k))[:20],
    }
    equal = not (result['unit_differences'] or result['function_differences'] or result['null_differences'])
    result['equal'] = equal
    print(json.dumps(result, indent=1, default=dict))
    return 0 if equal else 1


if __name__ == '__main__':
    raise SystemExit(main())
