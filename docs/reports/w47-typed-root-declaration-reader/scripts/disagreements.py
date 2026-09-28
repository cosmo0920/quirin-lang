"""Classify every `root-declarations` disagreement by the AST's own reason.

Usage: disagreements.py PARITY_JSONL OUT_JSON

De-duplicates rows by (source, function, full record). For each disagreement
kind, counts names by the AST-side cause: its binding reason (`ast_binding`),
`not-answered` (declared more than once, not automatic, or another object of
the spelling), or `none` (the AST answers never bound). For
`ast-binds-lexical-never-bound` it also lists, per cause, whether the name is
the root of an admitted typed position row of the same function (a verdict
that reads the lexical answer the AST disputes).
"""
import collections, json, re, sys
records = {}
admitted = collections.defaultdict(set)
for line in open(sys.argv[1]):
    r = json.loads(line)
    if r.get('position') == 'root-declarations':
        key = (r['source_sha256'], r['function'], json.dumps(r['names'], sort_keys=True))
        records[key] = r
    elif r.get('new_admitted') and r.get('old_route') == 'old_refusal':
        roots = set(re.findall(r'root: "([^"]+)"', r.get('new_chains') or ''))
        for group in re.findall(r'\bbases: \[([^\]]*)\]', r.get('new_scalar') or ''):
            roots |= set(re.findall(r'"([^"]+)"', group))
        admitted[(r['source_sha256'], r['function'])] |= roots
kinds = collections.defaultdict(collections.Counter)
exposure = collections.defaultdict(collections.Counter)
examples = collections.defaultdict(lambda: collections.defaultdict(list))
for (sha, fn, _), r in records.items():
    by_name = {n['name']: n for n in r['names']}
    for d in r['disagreements']:
        n = by_name[d['name']]
        cause = n['ast_binding'] or ('not-answered' if not n['ast_answered'] else 'none')
        for kind in d['kinds']:
            kinds[kind][cause] += 1
            if len(examples[kind][cause]) < 12:
                examples[kind][cause].append(f"{fn}:{d['name']}")
            if kind == 'ast-binds-lexical-never-bound':
                used = d['name'] in admitted[(sha, fn)]
                exposure[cause]['root of an admitted typed row' if used else 'not a typed-row root'] += 1
out = {'records': len(records),
       'kinds': {k: dict(v) for k, v in sorted(kinds.items())},
       'ast_binds_lexical_never_bound_exposure': {k: dict(v) for k, v in sorted(exposure.items())},
       'examples': {k: {c: v for c, v in d.items()} for k, d in sorted(examples.items())}}
json.dump(out, open(sys.argv[2], 'w'), indent=1)
print(json.dumps({k: out[k] for k in ('records', 'kinds', 'ast_binds_lexical_never_bound_exposure')}, indent=1))
