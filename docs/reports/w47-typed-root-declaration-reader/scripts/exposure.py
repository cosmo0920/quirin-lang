"""List every admitted typed row whose root the AST binds but the lexical closure calls never bound.

Usage: exposure.py PARITY_JSONL DIAGNOSTICS_JSON OUT_JSON
For each (function, root) disagreement of kind `ast-binds-lexical-never-bound`,
the admitted typed rows (P2 statements and P3 positions admitted by the typed
reader) naming that root, whether a chain from it carries
`root_never_bound: true`, the AST binding reason, and the function's status.
"""
import collections, json, re, sys
disputed = {}
rows = collections.defaultdict(list)
for line in open(sys.argv[1]):
    r = json.loads(line)
    if r.get('position') == 'root-declarations':
        by_name = {n['name']: n for n in r['names']}
        for d in r['disagreements']:
            if 'ast-binds-lexical-never-bound' in d['kinds']:
                n = by_name[d['name']]
                disputed[(r['function'], d['name'])] = n['ast_binding'] or (
                    'not-answered' if not n['ast_answered'] else 'none')
    elif r.get('new_admitted') and r.get('old_route') == 'old_refusal':
        chains = r.get('new_chains') or ''
        scalar = r.get('new_scalar') or ''
        roots = set(re.findall(r'root: "([^"]+)"', chains))
        for group in re.findall(r'\bbases: \[([^\]]*)\]', scalar):
            roots |= set(re.findall(r'"([^"]+)"', group))
        for root in roots:
            never = re.findall(r'root: "' + re.escape(root) + r'", dereferenced_root: (\w+), root_never_bound: (\w+)', chains)
            rows[(r['function'], root)].append({
                'line': r['line'], 'position': r.get('position', 'statement'),
                'texts': ' '.join(r['texts']), 'chains': [list(x) for x in never]})
status = {}
for unit in json.load(open(sys.argv[2]))['units']:
    for f in unit.get('functions', []):
        status.setdefault(f['function'], set()).add(f['status'])
out = []
for (fn, name), reason in sorted(disputed.items()):
    admitted = rows.get((fn, name))
    if not admitted:
        continue
    uniq = {json.dumps(a, sort_keys=True) for a in admitted}
    out.append({'function': fn, 'root': name, 'ast_binding': reason,
                'status': sorted(status.get(fn, [])),
                'rows': [json.loads(u) for u in sorted(uniq)]})
json.dump(out, open(sys.argv[3], 'w'), indent=1)
tally = collections.Counter()
for o in out:
    safe = 'Safe' in o['status']
    nb = any(c[1] == 'true' for row in o['rows'] for c in row['chains'])
    tally[(o['ast_binding'], 'Safe' if safe else 'non-Safe', 'root_never_bound-chain' if nb else 'no-never-bound-chain')] += 1
print('disputed roots with admitted typed rows:', len(out))
for k, v in sorted(tally.items()):
    print(' ', k, v)
