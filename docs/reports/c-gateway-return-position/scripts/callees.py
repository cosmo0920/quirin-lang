#!/usr/bin/env python3
"""Who are the callees of the 511 set's `return CALL(...)` leaves?

The honesty question of card C-GATEWAY-RETURN-POSITION sub-shape B: clearing a
return refusal on a call whose callee is uncontracted merely exchanges one
refusal for another. This measures, over the real population rather than a
sample, how many of those callees are (a) declared in the .qcore contract
corpus, (b) defined in the same analysed unit so that W4 could derive a
validated summary, or (c) neither.

Worktree-private measurement output; see CLAUDE.md hygiene rule.
"""
import gzip
import json
import re
import collections
import os

SRC = '/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source'
CONTRACTS = 'contract/qcore'
BASE = 'docs/reports/w31-corpus-contracts-2/artifacts/diagnostics.json.gz'


def declared_names():
    names = set()
    for dirpath, _dirs, files in os.walk(CONTRACTS):
        for fn in files:
            if not fn.endswith('.qcore'):
                continue
            text = open(os.path.join(dirpath, fn), errors='replace').read()
            # strip line comments so a name quoted in prose is not counted
            text = re.sub(r'//[^\n]*', '', text)
            for m in re.finditer(r'\bextern\s+fn\s+([A-Za-z_]\w*)', text):
                names.add(m.group(1))
    return names


def uncontracted_reported():
    """Callee names the baseline itself reports as uncontracted, corpus-wide."""
    d = json.load(gzip.open(BASE, 'rt'))
    seen = collections.Counter()
    for u in d['units']:
        for f in u['functions']:
            for g in f['diagnostics']:
                m = re.match(r'^call to uncontracted function `([^`]+)`$',
                             g['description'])
                if m:
                    seen[m.group(1)] += 1
    return seen


def main():
    rows = json.load(open('measure/c-gateway-return-position/shape-census.json'))
    calls = [r for r in rows if r['shape'] == 'call']
    decl = declared_names()
    unc = uncontracted_reported()

    for r in calls:
        m = re.search(r'\breturn\b\s*\(?\s*([A-Za-z_]\w*)\s*\(', r['text'])
        r['callee'] = m.group(1) if m else None

    named = [r for r in calls if r['callee']]
    buckets = collections.Counter()
    examples = collections.defaultdict(list)
    for r in named:
        c = r['callee']
        if c in decl:
            b = 'declared-contract'
        elif c.startswith('__') or c.startswith('__atomic') or c in (
                'malloc', 'calloc', 'realloc', 'free', 'memcpy', 'strdup'):
            b = 'libc-or-compiler-builtin'
        elif c in unc:
            b = 'reported-uncontracted-elsewhere'
        else:
            b = 'not-otherwise-seen'
        buckets[b] += 1
        examples[b].append((r['origin'], r['function'], c))

    print('return-position leaves classified `call`: %d' % len(calls))
    print('with an extractable callee name:        %d' % len(named))
    print()
    for b, n in buckets.most_common():
        print('%-34s %5d  %5.1f%%' % (b, n, 100.0 * n / len(named)))
    print()
    for b, _n in buckets.most_common():
        print('--- %s ---' % b)
        for e in examples[b][:8]:
            print('   %-42s %-38s %s' % e)
    print()
    top = collections.Counter(r['callee'] for r in named)
    print('most frequent callees in the 511 set:')
    for c, n in top.most_common(20):
        print('  %4d  %-44s declared=%s' % (n, c, c in decl))
    json.dump(named, open('measure/c-gateway-return-position/callees.json', 'w'),
              indent=1)


if __name__ == '__main__':
    main()
