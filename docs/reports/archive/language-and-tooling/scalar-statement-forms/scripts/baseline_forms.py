"""The five families in the measured routes of a published baseline.

Usage: baseline_forms.py BASELINE_DIR
"""
import collections as C, json, sys

B = sys.argv[1].rstrip('/') + '/'
F = {'unsupported assignment expression', 'unrecognized assignment destination',
     'braced initializer, expression, or declaration is not modeled',
     'unmodeled increment or decrement expression',
     'C operator `sizeof` requires type/operand analysis'}
for route, path in [('corpus', 'eight-home/corpus/corpus/resource-diagnostics.json'),
                    ('system', 'fluent-bit-system/system/resource-diagnostics.json'),
                    ('slice', 'fluent-bit-tail-pause/slice/resource-diagnostics.json')]:
    d = json.load(open(B + path))
    st = C.Counter()
    leaves = C.Counter()
    only = []
    for u in d['units']:
        for f in u['functions']:
            st[f['status']] += 1
            ds = [x['description'] for x in f.get('diagnostics', [])]
            for x in ds:
                if x in F:
                    leaves[x] += 1
            if ds and all(x in F for x in ds):
                only.append((u['file'], f['function'], f['status'],
                             [(x['description'], x.get('origin'), x['line']) for x in f['diagnostics']]))
    print(route, dict(st), dict(leaves), 'sole', len(only))
    for o in only:
        print('   ', o)
