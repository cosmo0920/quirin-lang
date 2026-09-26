"""Leaf-by-leaf difference between a run's JSON sidecar and an archived (gzip) one,
after rewriting ROOT to CANONICAL. Usage: json_leaf_diff.py RUN.json ARCHIVED.json.gz ROOT=CANONICAL

A disclosure beside baseline_identity.py, never a substitute for it."""
import gzip, json, sys
run = json.load(open(sys.argv[1]))
base = json.loads(gzip.decompress(open(sys.argv[2], 'rb').read()))
root, canon = sys.argv[3].split('=')
run = json.loads(json.dumps(run).replace(root, canon))

def diff(a, b, path='', out=None, limit=20):
    if out is None:
        out = []
    if len(out) >= limit:
        return out
    if type(a) != type(b):
        out.append((path, 'type', str(a)[:80], str(b)[:80]))
    elif isinstance(a, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                out.append((path + '/' + k, 'missing-in-' + ('run' if k not in a else 'base'), '', ''))
            else:
                diff(a[k], b[k], path + '/' + k, out, limit)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append((path, 'len', len(a), len(b)))
        for i, (x, y) in enumerate(zip(a, b)):
            diff(x, y, f'{path}[{i}]', out, limit)
    elif a != b:
        out.append((path, 'value', str(a)[:100], str(b)[:100]))
    return out

for item in diff(run, base):
    print(item)
