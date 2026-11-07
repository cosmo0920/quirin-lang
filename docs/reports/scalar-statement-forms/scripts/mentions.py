"""Rows of a report whose diagnostics name one of the given functions.

Usage: mentions.py REPORT NAME...
"""
import json, sys

d = json.load(open(sys.argv[1]))
names = sys.argv[2:]
for u in d['units']:
    for f in u['functions']:
        hits = [x for x in f.get('diagnostics', []) if any('`' + n + '`' in x['description'] or n == x.get('variable') for n in names)]
        if hits or f['function'] in names:
            print(u['file'], f['function'], f['status'],
                  sorted({x['description'][:90] for x in f.get('diagnostics', [])})[:6])
