"""Structural diff after normalising both sides. Usage: normdiff.py OLD NEW LIMIT [OLD_FROM=TO ...] -- [NEW_FROM=TO ...]"""
import json, sys
from collections import Counter
args = sys.argv[1:]
old_path, new_path, limit = args[0], args[1], int(args[2])
rest = args[3:]
split = rest.index('--')
old = open(old_path).read()
new = open(new_path).read()
for a in rest[:split]:
    x, y = a.split('=', 1)
    old = old.replace(x, y)
for a in rest[split + 1:]:
    x, y = a.split('=', 1)
    new = new.replace(x, y)
old, new = json.loads(old), json.loads(new)
out = []
kinds = Counter()


def walk(a, b, path):
    if type(a) != type(b):
        out.append((path, repr(a)[:160], repr(b)[:160]))
    elif isinstance(a, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                out.append((path + '/' + k, 'present' if k in a else 'absent', 'present' if k in b else 'absent'))
            else:
                walk(a[k], b[k], path + '/' + k)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append((path, f'len {len(a)}', f'len {len(b)}'))
        for i, (x, y) in enumerate(zip(a, b)):
            walk(x, y, f'{path}[{i}]')
    elif a != b:
        out.append((path, repr(a)[:160], repr(b)[:160]))


walk(old, new, '')
import re
for path, _, _ in out:
    kinds[re.sub(r'\[\d+\]', '[]', re.sub(r'/contracts/[^/]+/', '/contracts/*/', path)).split('/')[-1]] += 1
for row in out[:limit]:
    print(*row, sep='\n   ')
print('total differences:', len(out))
print('by last key:', dict(kinds))
