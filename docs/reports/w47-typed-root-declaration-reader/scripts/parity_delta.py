"""Older parity rows (every row but `root-declarations`) BEFORE vs AFTER, as multisets.

Usage: parity_delta.py BEFORE_PARITY AFTER_PARITY OUT_JSON
"""
import collections, json, sys


def load(path):
    rows = collections.Counter()
    roots = 0
    for line in open(path):
        r = json.loads(line)
        if r.get('position') == 'root-declarations':
            roots += 1
            continue
        rows[line] += 1
    return rows, roots


before, _ = load(sys.argv[1])
after, roots = load(sys.argv[2])
gone = before - after
new = after - before
summary = collections.Counter()
detail = []
for sign, bag in (('before-only', gone), ('after-only', new)):
    for line, n in bag.items():
        r = json.loads(line)
        key = f"{sign}|{r.get('position', 'statement')}|{r['old_route']}|admitted={r['new_admitted']}"
        summary[key] += n
        detail.append({'side': sign, 'count': n, 'function': r['function'], 'line': r['line'],
                       'position': r.get('position', 'statement'), 'texts': ' '.join(r['texts']),
                       'new_admitted': r['new_admitted']})
out = {'before_rows': sum(before.values()), 'after_rows_excluding_root_declarations': sum(after.values()),
       'root_declaration_rows': roots, 'delta': dict(sorted(summary.items())), 'rows': detail}
json.dump(out, open(sys.argv[3], 'w'), indent=1)
print(json.dumps({k: v for k, v in out.items() if k != 'rows'}, indent=1))
