"""attribute.py BEFORE AFTER PARITY.json OUT.json -- RDHTTP-HEADER-SUMMARY-GAP: attribute
every report difference between two corpus runs. Per unit (library, file):
every row whose status or diagnostics moved (leaves removed / added by
description), every other differing row key, every differing unit key; and
the parity comparison's differing functions grouped by unit file (read from
parity.py's `groups`). Prints a summary and writes the full table."""
import collections
import json
import pathlib
import sys


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / 'resource-diagnostics.json'
    return json.loads(path.read_bytes())


def leaf(d):
    return (d.get('category'), d.get('description'), d.get('origin'))


before, after = load(sys.argv[1]), load(sys.argv[2])
parity = json.load(open(sys.argv[3]))
units_b = {(u['library'], u['file']): u for u in before['units']}
units_a = {(u['library'], u['file']): u for u in after['units']}
assert units_b.keys() == units_a.keys(), 'unit sets differ'
table = {}
for key in units_b:
    ub, ua = units_b[key], units_a[key]
    entry = {}
    unit_keys = sorted(k for k in set(ub) | set(ua) if k != 'functions' and ub.get(k) != ua.get(k))
    if unit_keys:
        entry['unit_keys'] = unit_keys
    rows_b = {r['function']: r for r in ub.get('functions') or []}
    rows_a = {r['function']: r for r in ua.get('functions') or []}
    if rows_b.keys() != rows_a.keys():
        entry['row_sets'] = [sorted(rows_b.keys() - rows_a.keys()), sorted(rows_a.keys() - rows_b.keys())]
    rows = {}
    for name in sorted(rows_b.keys() & rows_a.keys()):
        rb, ra = rows_b[name], rows_a[name]
        if rb == ra:
            continue
        row = {}
        if rb.get('status') != ra.get('status'):
            row['status'] = [rb.get('status'), ra.get('status')]
        lb = collections.Counter(leaf(d) for d in rb.get('diagnostics') or [])
        la = collections.Counter(leaf(d) for d in ra.get('diagnostics') or [])
        if lb != la:
            row['removed'] = sorted(map(list, (lb - la).elements()))
            row['added'] = sorted(map(list, (la - lb).elements()))
        others = sorted(k for k in set(rb) | set(ra) if k != 'diagnostics' and rb.get(k) != ra.get(k))
        if others:
            row['other_keys'] = others
        rows[name] = row
    if rows:
        entry['rows'] = rows
    if entry:
        table['/'.join(key)] = entry
groups = parity.get('groups')
json.dump({'report': table, 'parity_groups': groups}, open(sys.argv[4], 'w'), indent=1, sort_keys=True)
for unit, entry in table.items():
    print(unit, 'unit keys', entry.get('unit_keys'))
    for name, row in (entry.get('rows') or {}).items():
        print('  ', name, row.get('status', ''), 'removed', len(row.get('removed', [])),
              'added', len(row.get('added', [])), 'other', row.get('other_keys', []))
