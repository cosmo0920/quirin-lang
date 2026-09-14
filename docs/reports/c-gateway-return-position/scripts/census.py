#!/usr/bin/env python3
"""BEFORE -> AFTER attribution for card C-GATEWAY-RETURN-POSITION.

Separates, as the wave-31 chain's tooling does, three disjoint indices off the
same documents: unit statuses, function statuses and leaves. A leaf's identity
is its INTERNAL preprocessed line (constraint 12a); the presented `origin`
coordinate is carried beside it for reading, never for identity.

It also answers the card's honesty requirement over the real 511 functions that
the baseline blocks on the return position ALONE: how many end with zero
leaves, how many merely EXCHANGE the return leaf for a different refusal (and
which), and how many are unmoved.

Usage: python3 measure/c-gateway-return-position/census.py \
           --before <run-dir> --after <run-dir>
"""
import argparse
import collections
import json
import gzip
import pathlib
import re

RETURN_LEAF = 'unmodeled call, aggregate or indirect access in return expression'
BASELINE = 'docs/reports/w31-corpus-contracts-2/artifacts/diagnostics.json.gz'


def load(path):
    path = pathlib.Path(path)
    if path.suffix == '.gz':
        return json.load(gzip.open(path, 'rt'))
    return json.load(open(path))


def inventory(doc):
    """Three disjoint indices: units, functions, leaves."""
    units, functions, leaves = {}, {}, set()
    origins = {}
    for u in doc['units']:
        key = (u.get('library'), u['file'])
        units[key] = u['status']
        for f in u['functions']:
            fkey = key + (f['function'],)
            functions[fkey] = f['status']
            for g in f['diagnostics']:
                leaf = fkey + (g['line'], g['category'], g['description'])
                leaves.add(leaf)
                origins[leaf] = g.get('origin')
    return units, functions, leaves, origins


def counts(doc):
    c = collections.Counter()
    for u in doc['units']:
        for f in u['functions']:
            for g in f['diagnostics']:
                c[g['category']] += 1
    return c


def statuses(before, after):
    moved = []
    for k in sorted(set(before) | set(after)):
        if before.get(k) != after.get(k):
            moved.append({'key': list(k), 'before': before.get(k),
                          'after': after.get(k)})
    return moved


def normalize(d):
    return re.sub(r'`[^`]*`', '`X`', d)


def return_only_set(doc):
    """The functions the document blocks on the return position and nothing else."""
    out = set()
    for u in doc['units']:
        for f in u['functions']:
            rs = [normalize(g['description']) for g in f['diagnostics']]
            if rs and set(rs) == {RETURN_LEAF}:
                out.add((u.get('library'), u['file'], f['function']))
    return out


def function_leaves(doc):
    out = {}
    for u in doc['units']:
        for f in u['functions']:
            out[(u.get('library'), u['file'], f['function'])] = [
                normalize(g['description']) for g in f['diagnostics']
            ]
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--before', required=True)
    ap.add_argument('--after', required=True)
    args = ap.parse_args()

    before = load(pathlib.Path(args.before) / 'resource-diagnostics.json')
    after = load(pathlib.Path(args.after) / 'resource-diagnostics.json')

    bu, bf, bl, borig = inventory(before)
    au, af, al, aorig = inventory(after)
    bc, ac = counts(before), counts(after)

    print('=' * 72)
    print('HEADLINE COUNTS')
    print('=' * 72)
    print('%-18s %10s %10s %8s' % ('category', 'before', 'after', 'delta'))
    for cat in sorted(set(bc) | set(ac)):
        d = ac[cat] - bc[cat]
        print('%-18s %10d %10d %+8d' % (cat, bc[cat], ac[cat], d))
    print('%-18s %10d %10d %+8d'
          % ('TOTAL', sum(bc.values()), sum(ac.values()),
             sum(ac.values()) - sum(bc.values())))

    print()
    print('units: %d -> %d   functions: %d -> %d'
          % (len(bu), len(au), len(bf), len(af)))

    def histogram(d):
        return collections.Counter(d.values())
    print('unit status before: %s' % dict(histogram(bu)))
    print('unit status after:  %s' % dict(histogram(au)))
    print('function status before: %s' % dict(histogram(bf)))
    print('function status after:  %s' % dict(histogram(af)))

    print()
    print('=' * 72)
    print('STATUS MOVES (separate from leaf moves)')
    print('=' * 72)
    um, fm = statuses(bu, au), statuses(bf, af)
    print('unit status moves: %d' % len(um))
    for m in um:
        print('   %s: %s -> %s' % ('/'.join(str(x) for x in m['key']),
                                   m['before'], m['after']))
    print('function status moves: %d' % len(fm))
    promotions = [m for m in fm if m['before'] != 'Safe' and m['after'] == 'Safe']
    withdrawals = [m for m in fm if m['before'] == 'Safe' and m['after'] != 'Safe']
    accusations = [m for m in fm if m['after'] == 'Unsafe' and m['before'] != 'Unsafe']
    print('   promotions to Safe:      %d' % len(promotions))
    print('   withdrawals from Safe:   %d' % len(withdrawals))
    print('   newly provisional Unsafe:%d' % len(accusations))
    for m in withdrawals + accusations:
        print('   %s: %s -> %s' % ('/'.join(str(x) for x in m['key']),
                                   m['before'], m['after']))

    print()
    print('=' * 72)
    print('LEAF MOVES')
    print('=' * 72)
    added, removed = sorted(al - bl), sorted(bl - al)
    print('leaves added:   %d' % len(added))
    print('leaves removed: %d' % len(removed))
    by_desc_removed = collections.Counter(x[5] for x in removed)
    by_desc_added = collections.Counter(x[5] for x in added)
    print()
    print('removed, by description:')
    for d, n in by_desc_removed.most_common(12):
        print('   %6d  %s' % (n, d[:90]))
    print('added, by description:')
    for d, n in by_desc_added.most_common(12):
        print('   %6d  %s' % (n, d[:90]))

    print()
    print('every ADDED leaf that is not `unsupported` (the accusing moves):')
    hard = [x for x in added if x[4] != 'unsupported']
    print('   count: %d' % len(hard))
    print('   %-14s %-28s %-40s %8s %-14s %s'
          % ('library', 'unit', 'function', 'line', 'category', 'origin'))
    for x in hard:
        print('   %-14s %-28s %-40s %8d %-14s %s'
              % (x[0], x[1][:28], x[2][:40], x[3], x[4], aorig.get(x, '?')))

    print()
    print('every REMOVED leaf that is not `unsupported`:')
    hardr = [x for x in removed if x[4] != 'unsupported']
    print('   count: %d' % len(hardr))
    for x in hardr:
        print('   %-14s %-28s %-40s %8d %-14s %s'
              % (x[0], x[1][:28], x[2][:40], x[3], x[4], borig.get(x, '?')))

    print()
    print('=' * 72)
    print('THE THREE-WAY SPLIT OF THE RETURN-ONLY-BLOCKED FUNCTIONS')
    print('  (the card\'s honesty requirement: clearing a return refusal that')
    print('   merely becomes a different refusal decides nothing)')
    print('=' * 72)
    target = return_only_set(before)
    af_leaves = function_leaves(after)
    bf_leaves = function_leaves(before)
    cleared, exchanged, unmoved, vanished = [], [], [], []
    exchange_reasons = collections.Counter()
    for k in sorted(target):
        if k not in af_leaves:
            vanished.append(k)
            continue
        a = af_leaves[k]
        if not a:
            cleared.append(k)
        elif RETURN_LEAF in a:
            unmoved.append(k)
        else:
            exchanged.append(k)
            for r in sorted(set(a)):
                exchange_reasons[r] += 1
    n = len(target)
    print('baseline return-only-blocked functions: %d' % n)
    print('  (1) end with ZERO leaves:          %5d  %5.1f%%'
          % (len(cleared), 100.0 * len(cleared) / n))
    print('  (2) EXCHANGE it for another leaf:  %5d  %5.1f%%'
          % (len(exchanged), 100.0 * len(exchanged) / n))
    print('  (3) unmoved (return leaf remains): %5d  %5.1f%%'
          % (len(unmoved), 100.0 * len(unmoved) / n))
    if vanished:
        print('  (-) no longer projected:           %5d' % len(vanished))
    print()
    print('what the exchanged functions now carry:')
    for r, c in exchange_reasons.most_common(12):
        print('   %5d  %s' % (c, r[:90]))

    print()
    print('of the functions that ended leaf-free, their AFTER status:')
    print('   %s' % dict(collections.Counter(
        af.get((k[0], k[1], k[2]), '?') for k in cleared)))
    print('NOTE: leaf-free is NOT Safe. Safe additionally requires every')
    print('obligation discharged and full analysed coverage at the certificate')
    print('gate (constraint 12); the histogram above is the measured answer.')

    out = {
        'counts_before': dict(bc), 'counts_after': dict(ac),
        'unit_status_moves': um, 'function_status_moves': fm,
        'leaves_added': [list(x) for x in added],
        'leaves_removed': [list(x) for x in removed],
        'return_only_split': {
            'baseline_population': n,
            'cleared': [list(k) for k in cleared],
            'exchanged': [list(k) for k in exchanged],
            'unmoved': [list(k) for k in unmoved],
            'exchange_reasons': dict(exchange_reasons),
        },
        'meaning': 'Diagnostic occurrences; Unsafe remains provisional. '
                   'Coordinates are internal preprocessed lines.',
    }
    json.dump(out, open('measure/c-gateway-return-position/census.json', 'w'),
              indent=1)


if __name__ == '__main__':
    main()
