"""K4 typedef census: which K4 formals are declared through a typedef of a
pointer (the parameter's declarator spells no `*`, the census types it a
pointer), per group, and which Unknown rows have only such K4 leaves.

usage: typedef_formals.py ROWS-1124.jsonl CENSUS_ARTIFACTS_DIR PREPARED_TREE OUT.json
Lexical: the signature is read from the original spelling at 046accf6.
"""
import collections, gzip, json, re, sys

K4 = {'formal-copied', 'formal-handed-contracted', 'formal-handed-uncontracted', 'formal-not-handed'}
rows = [json.loads(l) for l in open(sys.argv[1])]
old = {}
with gzip.open(sys.argv[2].rstrip('/') + '/alias-leaves.jsonl.gz', 'rt') as f:
    next(f)
    for l in f:
        r = json.loads(l)
        old.setdefault((r['file'], r['function'], r['variable'], r['origin']), r)
src = sys.argv[3].rstrip('/') + '/'
cache = {}


def signature(path, fn, line):
    if path not in cache:
        try:
            cache[path] = open(src + path, errors='replace').read().split('\n')
        except OSError:
            cache[path] = []
    t = cache[path]
    pat = re.compile(r'(^|[^\w.>])' + re.escape(fn) + r'\s*\(')
    for i in range(min(line, len(t)) - 1, -1, -1):
        if pat.search(t[i]) and not t[i].rstrip().endswith(';'):
            s = '\n'.join(t[i:i + 12])
            m = pat.search(s)
            j = m.end()
            depth = 1
            k = j
            while k < len(s) and depth:
                depth += {'(': 1, ')': -1}.get(s[k], 0)
                k += 1
            return s[j:k - 1]
    return None


def param_decl(sig, v):
    for p in re.split(r',(?![^()]*\))', sig or ''):
        if re.search(r'(?<!\w)' + re.escape(v) + r'\s*(\[[^\]]*\])?\s*$', p.strip()):
            return p.strip()
    return None


kind_by_group = collections.defaultdict(collections.Counter)
typedef_names = collections.Counter()
row_kinds = {}
for r in rows:
    if r['status'] != 'Unknown':
        continue
    rid = (r['u'], r['file'], r['fn'])
    ks = []
    for d in r['ds']:
        if d['c'] != 'unknown_alias':
            ks.append('other')
            continue
        m = re.search(r'`([^`]*)`', d['d'] or '')
        v = m.group(1) if m else '?'
        leaf = old.get((r['file'], r['fn'], v, d['o']))
        if not leaf or leaf['group'] not in K4:
            ks.append('other')
            continue
        p = param_decl(signature(d['o'].rsplit(':', 1)[0], r['fn'], int(d['o'].rsplit(':', 1)[1])), v)
        if p is None:
            k = 'unread'
        elif '*' in p or '[' in p:
            k = 'spelled-pointer'
        else:
            k = 'typedef-pointer'
            typedef_names[re.sub(r'\b' + re.escape(v) + r'\b', '', p).replace('const', '').strip()] += 1
        kind_by_group[leaf['group']][k] += 1
        ks.append('k4:' + k)
    row_kinds[rid] = ks
alone_typedef = [list(k) for k, ks in row_kinds.items() if ks and all(x == 'k4:typedef-pointer' for x in ks)]
res = dict(kind_by_group=kind_by_group, typedef_types=typedef_names.most_common(40),
           rows_alone_typedef_only=alone_typedef)
json.dump(res, open(sys.argv[4], 'w'), indent=1)
print(json.dumps(res, indent=1)[:4000])
