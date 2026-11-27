#!/usr/bin/env python3
"""Itemize every diagnostic and verdict change between two gateway reports."""
import collections
import json
import pathlib
import sys


def rows(path):
    report = json.loads(pathlib.Path(path).read_text())
    diagnostics = collections.Counter()
    functions = {}
    units = {}
    for unit in report['units']:
        key = (unit['library'], unit['file'])
        assert key not in units, key
        units[key] = unit['status']
        for function in unit['functions']:
            fkey = (*key, function['function'])
            assert fkey not in functions, fkey
            functions[fkey] = function['status']
            for diagnostic in function['diagnostics']:
                diagnostics[(*fkey, diagnostic['line'], diagnostic['category'],
                             diagnostic['description'])] += 1
    return diagnostics, functions, units


def changes(before, after):
    return [{'key': key, 'before': before.get(key), 'after': after.get(key)}
            for key in sorted(before.keys() | after.keys())
            if before.get(key) != after.get(key)]


def expanded(counter):
    return [{'library': row[0], 'file': row[1], 'function': row[2],
             'line': row[3], 'category': row[4], 'description': row[5], 'count': n}
            for row, n in sorted(counter.items())]


if __name__ == '__main__':
    bd, bf, bu = rows(sys.argv[1])
    ad, af, au = rows(sys.argv[2])
    print(json.dumps({
        'before': sys.argv[1], 'after': sys.argv[2],
        'removed': expanded(bd - ad), 'added': expanded(ad - bd),
        'functions': changes(bf, af), 'units': changes(bu, au),
    }, indent=2))
