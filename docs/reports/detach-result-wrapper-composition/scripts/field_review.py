#!/usr/bin/env python3
"""Leaf-level BASE/TIP field review of the corpus resource diagnostics.

Usage: field_review.py BASE_ROUTE_DIR TIP_ROUTE_DIR OUT_JSON

Every differing leaf of every row and unit is classified by its key path
(list indices erased). For each key the review records the distinct
(old, new) pairs. A key whose pairs form a single consistent substitution
(each old value maps to one new value across the whole corpus) is an
identity substitution, not a semantic change; every other key is listed with
all of its rows. Nothing is erased: the raw pairs are written out. Read-only.
"""
import collections
import json
import pathlib
import sys


def leaves(value, path=()):
    if isinstance(value, dict):
        for key in sorted(value):
            yield from leaves(value[key], path + (key,))
    elif isinstance(value, list):
        for item in value:
            yield from leaves(item, path + ('[]',))
    else:
        yield path, value


def diff(before, after, path=()):
    """Paired leaf differences; a shape change is reported whole."""
    if type(before) is not type(after):
        yield path, json.dumps(before, sort_keys=True), json.dumps(after, sort_keys=True)
    elif isinstance(before, dict):
        for key in sorted(set(before) | set(after)):
            if key not in before or key not in after:
                yield path + (key,), json.dumps(before.get(key), sort_keys=True), json.dumps(after.get(key), sort_keys=True)
            else:
                yield from diff(before[key], after[key], path + (key,))
    elif isinstance(before, list):
        if len(before) != len(after):
            yield path, json.dumps(before, sort_keys=True), json.dumps(after, sort_keys=True)
        else:
            for b, a in zip(before, after):
                yield from diff(b, a, path + ('[]',))
    elif before != after:
        yield path, before, after


def load(root):
    data = json.loads((pathlib.Path(root) / 'corpus/resource-diagnostics.json').read_text())
    units = {f"{u['library']}/{u['file']}": u for u in data['units']}
    return data, units


def main():
    base, base_units = load(sys.argv[1])
    tip, tip_units = load(sys.argv[2])
    pairs = collections.defaultdict(lambda: collections.defaultdict(set))
    rows = collections.defaultdict(set)
    for name in sorted(set(base_units) | set(tip_units)):
        b, t = base_units.get(name, {}), tip_units.get(name, {})
        b_rows = {r['function']: r for r in b.get('functions', [])}
        t_rows = {r['function']: r for r in t.get('functions', [])}
        for function in sorted(set(b_rows) | set(t_rows)):
            for path, old, new in diff(b_rows.get(function), t_rows.get(function)):
                key = 'row:' + '.'.join(path)
                pairs[key][json.dumps(old)].add(json.dumps(new))
                rows[key].add(f'{name}::{function}')
        b_unit = {k: v for k, v in b.items() if k != 'functions'}
        t_unit = {k: v for k, v in t.items() if k != 'functions'}
        for path, old, new in diff(b_unit, t_unit):
            key = 'unit:' + '.'.join(path)
            pairs[key][json.dumps(old)].add(json.dumps(new))
            rows[key].add(name)
    for path, old, new in diff({k: v for k, v in base.items() if k != 'units'},
                               {k: v for k, v in tip.items() if k != 'units'}):
        key = 'top:' + '.'.join(path)
        pairs[key][json.dumps(old)].add(json.dumps(new))
        rows[key].add('<document>')
    review = {}
    for key in sorted(pairs):
        mapping = pairs[key]
        consistent = all(len(news) == 1 for news in mapping.values())
        inverse = collections.Counter(next(iter(n)) for n in mapping.values() if len(n) == 1)
        injective = consistent and all(c == 1 for c in inverse.values())
        review[key] = dict(substitution=consistent and injective, distinct_pairs=sum(len(v) for v in mapping.values()),
                           places=len(rows[key]),
                           pairs=[[old, sorted(news)] for old, news in sorted(mapping.items())][:200],
                           pairs_truncated=sum(len(v) for v in mapping.values()) > 200,
                           where=sorted(rows[key]) if not (consistent and injective) else sorted(rows[key])[:20])
    out = dict(keys=review,
               semantic_keys=sorted(k for k, v in review.items() if not v['substitution']),
               substitution_keys=sorted(k for k, v in review.items() if v['substitution']))
    pathlib.Path(sys.argv[3]).write_text(json.dumps(out, indent=1, sort_keys=True) + '\n')
    print(json.dumps(dict(semantic_keys={k: review[k]['places'] for k in out['semantic_keys']},
                          substitution_keys={k: review[k]['places'] for k in out['substitution_keys']}), indent=1))


if __name__ == '__main__':
    raise SystemExit(main())
