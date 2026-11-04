"""Libc/builtin uncontracted-callee census over one resource-diagnostics report.

usage: gap.py REPORT LIBC_NAMES [--rows NAMES_FILE]

Prints, per libc/builtin callee reported as `call to uncontracted function`,
its (function, callee) pairs, sites, functions whose every diagnostic is an
uncontracted call ("only"), and the units' libraries. With --rows, prints
every function row that names one of the listed callees: status, leaf count,
and its uncontracted callees.
"""
import collections
import json
import re
import sys

report, names_path = sys.argv[1], sys.argv[2]
rows_names = None
if '--rows' in sys.argv:
    rows_names = set(open(sys.argv[sys.argv.index('--rows') + 1]).read().split())
d = json.load(open(report))
libc = {l.strip() for l in open(names_path) if l.strip() and not l.startswith('#')}
pat = re.compile(r'call to uncontracted function `([^`]+)`')
pairs = collections.defaultdict(set)
sites = collections.Counter()
libs = collections.defaultdict(set)
only = collections.defaultdict(set)
defined = set()
for u in d['units']:
    defined.update(u.get('source_functions', []))
for u in d['units']:
    for f in u['functions']:
        key = (u['file'], f['function'])
        names = set()
        other = False
        for dg in f.get('diagnostics', []):
            m = pat.search(dg['description'])
            if m:
                c = m.group(1)
                names.add(c)
                sites[c] += 1
                libs[c].add(u.get('library', '?'))
                pairs[c].add(key)
            else:
                other = True
        if names and not other:
            for c in names:
                only[c].add(key)
        if rows_names is not None and names & rows_names:
            print(f"ROW {u['file']}::{f['function']} {f['status']} leaves={len(f.get('diagnostics', []))} "
                  f"other={sum(1 for dg in f.get('diagnostics', []) if not pat.search(dg['description']))} "
                  f"unc={sorted(names)}")
if rows_names is None:
    rows = sorted(((len(pairs[c]), c) for c in pairs
                   if c in libc or c.startswith('__builtin')), reverse=True)
    total = 0
    for n, c in rows:
        total += n
        body = 'body-in-run' if c in defined else 'external'
        print(f"{c:28s} pairs {n:3d} sites {sites[c]:3d} only {len(only[c]):2d} {body:11s} libs {','.join(sorted(libs[c]))}")
    print('names', len(rows), 'pairs', total)
