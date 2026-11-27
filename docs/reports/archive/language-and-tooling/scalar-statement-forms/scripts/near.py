"""Rows whose leaves are the five families plus only unresolved-identity /
unknown-alias leaves (which a refusal's poisoned frame can cause).

Usage: near.py REPORT
"""
import json, sys

F = {'unsupported assignment expression', 'unrecognized assignment destination',
     'braced initializer, expression, or declaration is not modeled',
     'unmodeled increment or decrement expression',
     'C operator `sizeof` requires type/operand analysis'}
d = json.load(open(sys.argv[1]))
for u in d['units']:
    for f in u['functions']:
        ds = f.get('diagnostics', [])
        fam = [x for x in ds if x['description'] in F]
        rest = [x for x in ds if x['description'] not in F]
        if fam and all(x['category'] == 'unknown_alias' or x['description'].startswith('unresolved resource identity') for x in rest) and rest:
            print(u['file'], f['function'], f['status'], sorted({x['description'][:70] for x in ds}))
