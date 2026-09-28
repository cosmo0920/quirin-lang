"""Summarise premise-only refusals (real plan refused, all-true premises admit).

Usage: premise_census.py AUDIT_LOG [OUT_JSON]
"""
import collections, json, sys
rows = {}
for line in open(sys.argv[1], errors='replace'):
    if line.startswith('W47TRP '):
        r = json.loads(line[7:])
        key = (r['function'], r['hook'], r['line'], ' '.join(r['texts']))
        rows[key] = r
by = collections.Counter()
functions = collections.defaultdict(set)
for (fn, hook, line, texts), r in rows.items():
    cls = f"{hook} readable={r['readable']} declared={r['declared']}"
    by[cls] += 1
    functions[cls].add(fn)
print('distinct premise-only rows:', len(rows))
for cls, n in sorted(by.items()):
    print(f'  {cls}: {n} rows in {len(functions[cls])} functions')
all_fns = set(fn for (fn, *_rest) in rows)
print('functions:', len(all_fns))
if len(sys.argv) > 2:
    json.dump({'rows': [dict(r, key=list(k)) for k, r in sorted(rows.items())],
               'classes': {c: {'rows': by[c], 'functions': sorted(functions[c])} for c in by}},
              open(sys.argv[2], 'w'), indent=1)
