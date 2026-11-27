#!/usr/bin/env python3
"""Join a loop-measure run's rows (rule 48's `list_bound`) to the census's 87
list-length loops (W45-FB-ENGINE-PROGRESS census, `expressible_as:
list-contract`), per loop and per variant.

usage: join.py CENSUS ROWS OUT

A census loop is matched by file, function and the loop keyword's original
line (`origin`). Every variant's row must be present; the outcome is the
variants' common outcome, or `variants-differ` (which the script reports and
does not resolve). Refuses to publish when a census loop has no row.
"""
import collections
import json
import sys


def main():
    census_path, rows_path, out_path = sys.argv[1:4]
    census = json.load(open(census_path))
    loops = [l for l in census['loops'] if l['expressible_as'] == 'list-contract']
    by_site = collections.defaultdict(list)
    for line in open(rows_path):
        row = json.loads(line)
        for fact in row['loops']:
            origin = fact.get('origin')
            if not origin:
                continue
            path, _, number = origin.rpartition(':')
            by_site[(path, row['function'], int(number))].append(fact)
    table = []
    missing = []
    for loop in loops:
        facts = by_site.get((loop['file'], loop['function'], loop['line']), [])
        if not facts:
            missing.append(loop['id'])
            continue
        outcomes = []
        for fact in facts:
            lb = fact.get('list_bound')
            if lb is None:
                outcomes.append(('no-list-outcome', None))
            elif lb['bound']:
                outcomes.append(('bounded', lb['bound']['list']))
            else:
                outcomes.append((lb['refusal'], lb['detail']))
        distinct = sorted(set(outcomes), key=str)
        outcome, detail = distinct[0] if len(distinct) == 1 else ('variants-differ', str(distinct))
        table.append({
            'id': loop['id'], 'site': loop['site'], 'function': loop['function'],
            'premises': loop['premises'], 'variants': len(facts),
            'outcome': outcome, 'detail': detail,
        })
    if missing:
        sys.exit(f'census loops with no row: {missing}')
    tally = collections.Counter(entry['outcome'] for entry in table)
    json.dump({'schema': 'w45-fb-list-finiteness-premise/join/v1',
               'loops': len(table), 'outcomes': dict(sorted(tally.items())),
               'table': table}, open(out_path, 'w'), indent=1)
    print(json.dumps(dict(sorted(tally.items()))))


if __name__ == '__main__':
    main()
