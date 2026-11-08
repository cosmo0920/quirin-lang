"""Rows whose every leaf is a five-family refusal or a call to one of the
predicted Safe functions (their callers could move with them).

Usage: callers.py REPORT NAME...
"""
import json, sys

F = {'unsupported assignment expression', 'unrecognized assignment destination',
     'braced initializer, expression, or declaration is not modeled',
     'unmodeled increment or decrement expression',
     'C operator `sizeof` requires type/operand analysis'}
names = sys.argv[2:]
calls = {f'call to uncontracted function `{n}`' for n in names}
d = json.load(open(sys.argv[1]))
for u in d['units']:
    for f in u['functions']:
        ds = [x['description'] for x in f.get('diagnostics', [])]
        if ds and any(x in calls for x in ds) and all(x in F or x in calls or x.startswith('unresolved resource identity') or x.startswith('unknown alias') for x in ds):
            print(u['file'], f['function'], f['status'], sorted(set(ds)))
