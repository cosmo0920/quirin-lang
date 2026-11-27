"""Summarise the `root-declarations` parity rows and tie every changed function to them.

Usage: root_rows.py PARITY_JSONL CHANGED_FUNCTIONS_JSONL_GZ OUT_JSON
"""
import collections, gzip, json, sys
rows = {}
for line in open(sys.argv[1]):
    r = json.loads(line)
    if r.get('position') != 'root-declarations':
        continue
    key = (r['source_sha256'], r['function'])
    body = json.dumps({k: r[k] for k in ('lexical_readable', 'ast_answers', 'served', 'names', 'disagreements')}, sort_keys=True)
    rows.setdefault(key, set()).add(body)
unstable = [k for k, v in rows.items() if len(v) != 1]
records = {k: json.loads(next(iter(v))) for k, v in rows.items()}
served = collections.Counter(r['served'] for r in records.values())
kinds = collections.Counter()
kind_functions = collections.defaultdict(set)
names_total = 0
for (sha, fn), r in records.items():
    names_total += len(r['names'])
    for d in r['disagreements']:
        for k in d['kinds']:
            kinds[k] += 1
            kind_functions[k].add(fn)
by_function = collections.defaultdict(set)
for (sha, fn), r in records.items():
    by_function[fn].add(r['served'])
changed = [json.loads(l) for l in gzip.open(sys.argv[2], 'rt')]
tie = collections.Counter()
changed_rows = []
for c in changed:
    s = sorted(by_function.get(c['function'], {'absent'}))
    tie['|'.join(s)] += 1
    changed_rows.append({'function': c['function'], 'library': c['library'], 'served': s,
                         'removed': c['removed'], 'added': c['added']})
out = {
    'distinct_function_records': len(records),
    'records_differing_between_analyses': len(unstable),
    'served': dict(served),
    'declared_names': names_total,
    'disagreement_kinds': {k: {'names': kinds[k], 'functions': len(kind_functions[k])} for k in sorted(kinds)},
    'changed_functions_by_served': dict(tie),
    'changed_functions': changed_rows,
    'examples': {k: sorted(kind_functions[k])[:15] for k in sorted(kinds)},
}
json.dump(out, open(sys.argv[3], 'w'), indent=1)
print(json.dumps({k: v for k, v in out.items() if k not in ('changed_functions', 'examples')}, indent=1))
