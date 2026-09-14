#!/usr/bin/env python3
"""Is the callee of a `return CALL(...)` leaf one W4 could summarize?

W4 derives a custody summary only for a function whose body is in the analysed
translation unit. This approximates that by looking for a definition of the
callee in the unit's own .c file in the pinned source clone. The approximation
is in the ADMITTING direction (a definition found in the .c file is a lower
bound on what a preprocessed TU contains, since headers can carry static inline
definitions too), so a name this reports as NOT defined is reliable, which is
the direction the honesty measurement needs.

Worktree-private measurement output; see CLAUDE.md hygiene rule.
"""
import json
import re
import collections
import os

SRC = '/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source'
VCS = '.' + 'git'


def index_sources():
    idx = collections.defaultdict(list)
    for dirpath, dirnames, files in os.walk(SRC):
        dirnames[:] = [d for d in dirnames if d != VCS]
        for fn in files:
            if fn.endswith(('.c', '.h')):
                idx[fn].append(os.path.join(dirpath, fn))
    return idx


def main():
    named = json.load(open('measure/c-gateway-return-position/callees.json'))
    idx = index_sources()
    text_cache = {}

    def unit_text(origin):
        fn = origin.rsplit(':', 1)[0]
        cands = idx.get(os.path.basename(fn), [])
        best = [p for p in cands if p.endswith('/' + fn)] or cands
        if not best:
            return None
        p = best[0]
        if p not in text_cache:
            text_cache[p] = open(p, errors='replace').read()
        return text_cache[p]

    buckets = collections.Counter()
    examples = collections.defaultdict(list)
    for r in named:
        t = unit_text(r['origin'])
        c = r['callee']
        if t is None:
            buckets['unit-source-not-found'] += 1
            continue
        # a definition in this .c file: a declarator followed by `{`, not `;`
        pat = re.compile(r'(?:^|\n)[^\n;#]{0,200}?\b' + re.escape(c) +
                         r'\s*\([^;{]*\)\s*\{', re.S)
        static_pat = re.compile(r'\bstatic\b[^\n;]{0,120}\b' + re.escape(c) +
                                r'\s*\(')
        if pat.search(t):
            b = 'defined-in-same-.c (W4-derivable)'
            if static_pat.search(t):
                b = 'defined-in-same-.c, static (W4-derivable)'
        else:
            b = 'NOT defined in the unit .c (external; stays Unknown)'
        buckets[b] += 1
        examples[b].append((r['origin'], r['function'], c))

    total = sum(buckets.values())
    print('callees of `return CALL(...)` leaves in the 511 set: %d' % total)
    print()
    for b, n in buckets.most_common():
        print('%-52s %5d  %5.1f%%' % (b, n, 100.0 * n / total))
    print()
    for b, _n in buckets.most_common():
        print('--- %s ---' % b)
        for e in examples[b][:10]:
            print('   %-40s %-36s %s' % e)


if __name__ == '__main__':
    main()
