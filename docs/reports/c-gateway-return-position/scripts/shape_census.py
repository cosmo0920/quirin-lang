#!/usr/bin/env python3
"""Shape census of the 511 return-only-blocked functions (report §4).

Reads the corpus baseline and the pinned Fluent Bit source clone, resolves each
return-position leaf's presented origin coordinate (constraint 12a: `origin` is
for reading, the internal preprocessed `line` is the identity) to its source
text, and classifies the returned expression.

The card supplied a 120-function sample as a lead; this measures the whole
population instead. Writes `shape-census.json` for `callees.py`.

Worktree-private measurement output; see CLAUDE.md hygiene rule.
"""
import gzip
import json
import re
import collections
import os

SRC = '/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source'
BASE = 'docs/reports/w31-corpus-contracts-2/artifacts/diagnostics.json.gz'
RET = 'unmodeled call, aggregate or indirect access in return expression'
VCS = '.' + 'git'
OUT = 'measure/c-gateway-return-position/shape-census.json'


def norm(s):
    return re.sub(r'`[^`]*`', '`X`', s)


def join_statement(lines, i, limit=12):
    """Join source lines from `i` until the statement's terminating `;`.

    Continuation is decided at paren/bracket depth 0 so a `;` inside a call
    argument list cannot terminate early. This is a census aid, never a parser.
    """
    out, depth = [], 0
    for k in range(i, min(i + limit, len(lines))):
        raw = lines[k]
        out.append(raw.strip())
        for ch in raw:
            if ch in '([':
                depth += 1
            elif ch in ')]':
                depth -= 1
            elif ch == ';' and depth <= 0:
                return ' '.join(out)
    return ' '.join(out)


def build_index(root):
    idx = collections.defaultdict(list)
    for dirpath, dirnames, files in os.walk(root):
        dirnames[:] = [d for d in dirnames if d != VCS]
        for fn in files:
            if fn.endswith(('.c', '.h')):
                idx[fn].append(os.path.join(dirpath, fn))
    return idx


def classify(t):
    if not t:
        return 'unresolved-source'
    m = re.search(r'\breturn\b\s*(.*?)\s*;\s*$', t)
    if not m:
        return ('return-not-complete-on-line' if 'return' in t
                else 'origin-line-is-not-a-return')
    e = m.group(1).strip()
    if e == '':
        return 'return-void'
    while e.startswith('(') and e.endswith(')') and e.count('(') == e.count(')'):
        inner = e[1:-1]
        if inner.count('(') != inner.count(')'):
            break
        e = inner.strip()
    if '?' in e:
        return 'conditional-expression'
    m2 = re.match(r'^([A-Za-z_]\w*)\s*\((.*)\)$', e)
    if m2 and m2.group(2).count('(') == m2.group(2).count(')'):
        return 'call'
    if e.startswith('&'):
        return 'address-of'
    if '[' in e:
        return 'subscript'
    if re.match(r'^[A-Za-z_]\w*(\s*(->|\.)\s*[A-Za-z_]\w*)+$', e):
        return 'field-chain'
    if e.startswith('*'):
        return 'deref'
    if re.match(r'^\([A-Za-z_][\w\s*]*\)\s*', e):
        return 'cast'
    if re.match(r'^[A-Za-z_]\w*$', e):
        return 'plain-name'
    return 'other'


def main():
    d = json.load(gzip.open(BASE, 'rt'))
    idx = build_index(SRC)
    cache, missing = {}, collections.Counter()

    def read_at(origin):
        m = re.match(r'^(.*):(\d+)$', origin or '')
        if not m:
            return None
        fn, ln = m.group(1), int(m.group(2))
        cands = idx.get(os.path.basename(fn), [])
        if not cands:
            missing[os.path.basename(fn)] += 1
            return None
        p = ([c for c in cands if c.endswith('/' + fn)] or cands)[0]
        if p not in cache:
            cache[p] = open(p, errors='replace').read().split('\n')
        lines = cache[p]
        return join_statement(lines, ln - 1) if 1 <= ln <= len(lines) else None

    recs = []
    for u in d['units']:
        for f in u['functions']:
            rs = [norm(g['description']) for g in f['diagnostics']]
            if rs and set(rs) == {RET}:
                recs.append((u['file'], f['function'], f['diagnostics']))

    rows = []
    for file, fname, gs in recs:
        for g in gs:
            text = read_at(g.get('origin'))
            rows.append({'unit': file, 'function': fname,
                         'origin': g.get('origin'), 'line': g['line'],
                         'text': (text or '').strip(),
                         'shape': classify(text)})

    cat = collections.Counter(r['shape'] for r in rows)
    by_cat = collections.defaultdict(list)
    for r in rows:
        by_cat[r['shape']].append(r)

    print('return-only-blocked functions: %d' % len(recs))
    print('return-position leaves in that set: %d' % len(rows))
    print('resolved to pinned source: %d' % sum(1 for r in rows if r['text']))
    if missing:
        print('unresolved basenames: %s' % dict(missing.most_common(10)))
    print()
    print('%-32s %6s %7s' % ('shape', 'count', 'pct'))
    for c, n in cat.most_common():
        print('%-32s %6d %6.1f%%' % (c, n, 100.0 * n / len(rows)))
    print()
    for c, _n in cat.most_common():
        print('--- %s ---' % c)
        for r in by_cat[c][:6]:
            print('   %-34s %-40s %s'
                  % (r['origin'], r['function'][:40], r['text'][:70]))
    json.dump(rows, open(OUT, 'w'), indent=1)
    print('\nwrote %s' % OUT)


if __name__ == '__main__':
    main()
