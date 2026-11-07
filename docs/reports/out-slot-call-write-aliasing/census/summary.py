"""Summarise the AST censuses: pointer out slots, and the calls after them.

usage: summary.py <census.json>... [-v]

A call is listed when one of its actuals is not excluded by its reach class
(`scalar`, `const`, `fnptr`, `null`, `string`, `addr-local`). `-v` also
lists the rows that store only through a scalar formal (no out slot).
"""
import collections as C
import json
import sys

EXCL = ('scalar', 'const', 'fnptr', 'null', 'string', 'addr-local')
verbose = '-v' in sys.argv
for path in [a for a in sys.argv[1:] if a != '-v']:
    d = json.load(open(path))
    rows = d['rows']
    errs = [r for r in rows if 'error' in r]
    print(f"== {d['route']}: {d['safe_rows']} rows read, {len(rows) - len(errs)} storing, "
          f"{len(errs)} unread {dict(C.Counter(r['error'] for r in errs))}")
    for r in errs:
        print('   unread', r['library'], r['file'], r['function'], r['error'])
    for r in rows:
        if 'error' in r:
            continue
        slots = [s for s in r['stores'] if s['type'].replace(' ', '').endswith('**')]
        if not slots and not verbose:
            continue
        print('  ', 'POINTER-SLOT' if slots else 'scalar-store', r['library'], r['file'], r['function'],
              sorted({(s['formal'], s['type'], s['value_type']) for s in r['stores']}))
        for c in r['calls']:
            reach = [a['reach'] for a in c['actuals'] if a['reach'] not in EXCL]
            if reach:
                print('       call after the slot:', c['callee'], reach)
