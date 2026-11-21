#!/usr/bin/env python3
"""STATUS-COMPACT: route totals of every integrator native comparison since the baseline.

Usage: landings.py FILE.json.gz...

Prints, per comparison and route, the before/after function status totals,
the number of status moves and the side specifications, read from the
committed `native-comparison.json.gz` records (primary landing evidence).
"""
import gzip
import json
import sys


def fmt(t):
    s = t.get('function_status', {})
    return '{} rows: {} Safe / {} Pending / {} Unknown / {} Unsafe'.format(
        t.get('function_rows'), s.get('Safe', 0), s.get('Pending', 0), s.get('Unknown', 0), s.get('Unsafe', 0))


for path in sys.argv[1:]:
    d = json.load(gzip.open(path))
    sides = {k: (v.get('spec') or v.get('revision')) for k, v in d.get('sides', {}).items()}
    print(f'== {path}')
    print(f'   sides {sides}; declared {d.get("declared") is not None}')
    for c in d.get('comparisons', []):
        for route, v in sorted(c['routes'].items()):
            t = v.get('totals') or {}
            if 'before' not in t:
                print(f'   {c["comparison"]} {route}: status {v.get("status")}')
                continue
            print(f'   {c["comparison"]} {route}: {fmt(t["before"])} -> {fmt(t["after"])}; moves {len(v.get("moves", []))}')
