"""K4 mention census: how the formal of each K4 leaf is mentioned in the
actuals of the calls the census names as uncontracted (or any call, for the
contracted groups), read lexically from the prepared tree at 046accf6.

usage: mentions.py ROWS-1124.jsonl CENSUS_ARTIFACTS_DIR PREPARED_TREE OUT.json

For every Unknown row of the 11-24 report with a K4 leaf, the (function,
formal) pairs are taken once. The function body is located by the leaf's
origin line (the definition preceding it); every call `NAME(` in the body
whose name is in the leaf's `uncontracted_callees` (formal-handed-uncontracted)
has its actuals split at top-level commas, and each actual mentioning the
formal is classified:
  bare      the actual is the formal, up to parentheses and casts
  member    the formal only as the base of `->` or `.` member reads (`p->f`, `p->a.b`)
  address   `&(p)->f`, `&p->f[...]`: a member address
  arith     `p + e`, `p - e`
  other     anything else (the formal inside a nested call, a subscript, ...)
A row's `blocked_by_call` is whether it also carries an uncontracted-call
leaf (`UNC`). Lexical only: macros are expanded in the prepared tree's
`.qcore` text only where the gateway expands them, so this reads the
original spelling and is a forecast, not evidence.
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


def text(path):
    if path not in cache:
        try:
            cache[path] = open(src + path, errors='replace').read().split('\n')
        except OSError:
            cache[path] = []
    return cache[path]


def body(path, fn, line):
    t = text(path)
    pat = re.compile(r'(^|[^\w.>])' + re.escape(fn) + r'\s*\(')
    start = None
    for i in range(min(line, len(t)) - 1, -1, -1):
        if pat.search(t[i]) and not t[i].rstrip().endswith(';'):
            start = i
            break
    if start is None:
        return ''
    depth, seen, out = 0, False, []
    for j in range(start, min(len(t), start + 2000)):
        out.append(t[j])
        depth += t[j].count('{') - t[j].count('}')
        if '{' in t[j]:
            seen = True
        if seen and depth <= 0:
            break
    return '\n'.join(out)


def actuals(s, i):
    """Split the argument list starting at s[i] == '(' into top-level actuals."""
    depth, cur, out = 0, [], []
    for j in range(i, len(s)):
        c = s[j]
        if c in '([{':
            depth += 1
            if depth == 1:
                continue
        elif c in ')]}':
            depth -= 1
            if depth == 0:
                out.append(''.join(cur).strip())
                return out
        elif c == ',' and depth == 1:
            out.append(''.join(cur).strip())
            cur = []
            continue
        cur.append(c)
    return out


def classify(a, v):
    w = re.compile(r'(?<![\w.>])' + re.escape(v) + r'(?!\w)')
    if not w.search(a):
        return None
    core = re.sub(r'^\(\s*[\w\s\*]+\)\s*', '', a.strip())
    core = core.strip()
    while core.startswith('(') and core.endswith(')'):
        core = core[1:-1].strip()
    if re.fullmatch(r'\(?\s*' + re.escape(v) + r'\s*\)?', core):
        return 'bare'
    if re.fullmatch(r'&\s*\(?\s*\(?' + re.escape(v) + r'\)?\s*(->|\.)[\w\[\]\.\->\s]*\)?', core):
        return 'address'
    if re.fullmatch(r'\(?\s*\(?' + re.escape(v) + r'\)?\s*(->|\.)[\w\.\->\s]*\)?', core):
        return 'member'
    if re.fullmatch(r'\(?\s*' + re.escape(v) + r'\s*[\+\-]\s*[\w\s\(\)\*\+\-]+', core):
        return 'arith'
    return 'other'


stats = collections.Counter()
by_row = collections.Counter()
samples = collections.defaultdict(list)
pairs_seen = set()
for r in rows:
    if r['status'] != 'Unknown':
        continue
    has_unc = any((d['d'] or '').startswith('call to uncontracted function') for d in r['ds'])
    for d in r['ds']:
        if d['c'] != 'unknown_alias':
            continue
        m = re.search(r'`([^`]*)`', d['d'] or '')
        v = m.group(1) if m else '?'
        leaf = old.get((r['file'], r['fn'], v, d['o']))
        if not leaf or leaf['group'] != 'formal-handed-uncontracted':
            continue
        key = (r['u'], r['file'], r['fn'], v)
        if key in pairs_seen:
            continue
        pairs_seen.add(key)
        callees = set(leaf['facts'].get('uncontracted_callees', []))
        b = body(d['o'].rsplit(':', 1)[0], r['fn'], int(d['o'].rsplit(':', 1)[1]))
        kinds = set()
        for cm in re.finditer(r'\b(\w+)\s*\(', b):
            if cm.group(1) not in callees:
                continue
            for a in actuals(b, cm.end() - 1):
                k = classify(a, v)
                if k:
                    kinds.add(k)
                    if len(samples[k]) < 12:
                        samples[k].append(f"{r['fn']}: {cm.group(1)}(... {a[:80]} ...)")
        tag = '+'.join(sorted(kinds)) or 'none-found'
        stats[(tag, 'UNC' if has_unc else 'noUNC')] += 1
res = dict(pairs=len(pairs_seen),
           by_kind=sorted(([k[0], k[1], n] for k, n in stats.items()), key=lambda x: -x[2]),
           samples=samples)
json.dump(res, open(sys.argv[4], 'w'), indent=1)
print(json.dumps(res['by_kind']))
for k, v in samples.items():
    print('==', k)
    for s in v[:6]:
        print('  ', s)
