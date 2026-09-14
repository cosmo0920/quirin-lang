#!/usr/bin/env python3
"""Read every moved leaf's own C back out of the pinned source.

A count is not a finding until someone has looked at what moved. This resolves
each of the card's moved leaves from its internal preprocessed line to its
presented origin coordinate (constraint 12a) and prints the C at that
coordinate, so the report can state what each admitted return actually is.
"""
import json
import os
import re
import collections

SRC = '/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source'
VCS = '.' + 'git'


def index():
    idx = collections.defaultdict(list)
    for dirpath, dirnames, files in os.walk(SRC):
        dirnames[:] = [d for d in dirnames if d != VCS]
        for fn in files:
            if fn.endswith(('.c', '.h')):
                idx[fn].append(os.path.join(dirpath, fn))
    return idx


def main():
    census = json.load(open('measure/c-gateway-return-position/census.json'))
    before = json.load(open(
        'measure/c-gateway-return-position/runs/before/resource-diagnostics.json'))

    # internal line -> presented origin, from the BEFORE document itself
    origin = {}
    for u in before['units']:
        for f in u['functions']:
            for g in f['diagnostics']:
                origin[(u.get('library'), u['file'], f['function'], g['line'])] = \
                    g.get('origin')

    idx = index()
    cache = {}

    def text_at(o):
        if not o:
            return None
        fn, ln = o.rsplit(':', 1)
        cands = idx.get(os.path.basename(fn), [])
        best = [p for p in cands if p.endswith('/' + fn)] or cands
        if not best:
            return None
        p = best[0]
        if p not in cache:
            cache[p] = open(p, errors='replace').read().split('\n')
        lines = cache[p]
        ln = int(ln)
        if not (1 <= ln <= len(lines)):
            return None
        out, depth = [], 0
        for k in range(ln - 1, min(ln + 7, len(lines))):
            out.append(lines[k].strip())
            for ch in lines[k]:
                if ch in '([':
                    depth += 1
                elif ch in ')]':
                    depth -= 1
                elif ch == ';' and depth <= 0:
                    return ' '.join(out)
        return ' '.join(out)

    shapes = collections.Counter()
    print('%-12s %-26s %-42s %-30s %s'
          % ('library', 'unit', 'function', 'origin', 'C at that coordinate'))
    for x in census['leaves_removed']:
        key = (x[0], x[1], x[2], x[3])
        o = origin.get(key)
        t = text_at(o) or ''
        m = re.search(r'\breturn\b\s*(.*?)\s*;', t)
        e = m.group(1) if m else ''
        if re.fullmatch(r'[A-Za-z_]\w*(\s*->\s*[A-Za-z_]\w*)+', e):
            shapes['pointer-valued `->` chain'] += 1
        elif e:
            shapes['other: ' + e[:40]] += 1
        else:
            shapes['unresolved'] += 1
        print('%-12s %-26s %-42s %-30s %s'
              % (x[0], x[1][:26], x[2][:42], o, t[:70]))
    print()
    print('shape of every moved leaf:')
    for s, n in shapes.most_common():
        print('   %3d  %s' % (n, s))


if __name__ == '__main__':
    main()
