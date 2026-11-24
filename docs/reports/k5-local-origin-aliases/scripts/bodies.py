"""Print the original source body of functions: bodies.py TREE file::fn::line ..."""
import re, sys
src = sys.argv[1].rstrip('/') + '/'
for a in sys.argv[2:]:
    path, fn, line = a.split('::')
    t = open(src + path, errors='replace').read().split('\n')
    line = int(line)
    pat = re.compile(r'(^|[^\w.>])' + re.escape(fn) + r'\s*\(')
    start = None
    for i in range(line - 1, -1, -1):
        if pat.search(t[i]) and not t[i].rstrip().endswith(';'):
            start = i
            break
    if start is None:
        print('?? no def', a)
        continue
    depth = 0
    seen = False
    out = []
    for j in range(start, min(len(t), start + 400)):
        out.append(f'{j+1:5d} {t[j]}')
        depth += t[j].count('{') - t[j].count('}')
        if '{' in t[j]:
            seen = True
        if seen and depth <= 0:
            break
    print('=====', a)
    print('\n'.join(out))
