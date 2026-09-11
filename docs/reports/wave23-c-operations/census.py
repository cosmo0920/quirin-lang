#!/usr/bin/env python3
"""Count diagnostic families in a recorded gateway audit (not distinct sites)."""
import collections
import gzip
import json
import pathlib
import sys


def census(path):
    path = pathlib.Path(path)
    opener = gzip.open if path.suffix == '.gz' else open
    with opener(path, 'rt', encoding='utf-8') as source:
        report = json.load(source)
    families = collections.Counter()
    callees = collections.Counter()
    functions = collections.Counter()
    for unit in report['units']:
        for function in unit['functions']:
            functions[function['status']] += 1
            for diagnostic in function['diagnostics']:
                if diagnostic['category'] != 'unsupported':
                    continue
                description = diagnostic['description']
                if description.startswith('call to uncontracted function '):
                    families['uncontracted call'] += 1
                    callees[description] += 1
                else:
                    families[description] += 1
    return {
        'input': str(path),
        'meaning': 'diagnostic occurrences; not unique C operations or safety coverage',
        'unit_statuses': dict(collections.Counter(u['status'] for u in report['units'])),
        'function_statuses': dict(functions),
        'diagnostic_counts': report['diagnostic_counts'],
        'unsupported_families': dict(families.most_common()),
        'uncontracted_calls': dict(callees.most_common()),
    }


if __name__ == '__main__':
    print(json.dumps(census(sys.argv[1]), indent=2) + '\n', end='')
