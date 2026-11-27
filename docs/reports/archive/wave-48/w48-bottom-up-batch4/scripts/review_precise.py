#!/usr/bin/env python3
"""Retain precise artifact differences and finite identity correspondence."""
from pathlib import Path
from collections import Counter
import difflib
import gzip
import hashlib
import json
import sys

ANALYZER = Path(__file__).resolve().parents[4]
ROOTS = [(str(ANALYZER / 'private/w48-bottom-up-batch4/inputs-base-final'), '$INPUTS'),
         (str(ANALYZER / 'private/w48-bottom-up-batch4/inputs-tip-final'), '$INPUTS'),
         (str(ANALYZER / 'private/w48-bottom-up-batch4/inputs-base'), '$INPUTS'),
         (str(ANALYZER / 'private/w48-bottom-up-batch4/inputs-tip'), '$INPUTS'),
         (str(ANALYZER), '$ANALYZER')]
OUT = Path(__file__).resolve().parents[1] / 'artifacts' / sys.argv[1]


def norm(x, mapping=None):
    if isinstance(x, str):
        for root, label in ROOTS:
            x = x.replace(root, label)
        return (mapping or {}).get(x, x)
    if isinstance(x, list):
        return [norm(y, mapping) for y in x]
    if isinstance(x, dict):
        result = {norm(k, mapping): norm(v, mapping) for k, v in x.items()}
        assert len(result) == len(x), 'normalization collision'
        return result
    return x


def diff(a, b, path=()):
    if a == b:
        return []
    if isinstance(a, dict) and isinstance(b, dict):
        result = []
        for k in sorted(a.keys() | b.keys()):
            if k not in a or k not in b:
                result.append(dict(path=path + (k,), before=a.get(k), after=b.get(k),
                                   before_present=k in a, after_present=k in b))
            else:
                result.extend(diff(a[k], b[k], path + (k,)))
        return result
    if isinstance(a, list) and isinstance(b, list) and len(a) == len(b):
        return [d for i, (x, y) in enumerate(zip(a, b)) for d in diff(x, y, path + (i,))]
    return [dict(path=path, before=a, after=b)]


def load(side, family):
    p = OUT / side / (family + '.json.gz')
    return json.loads(gzip.decompress(p.read_bytes())) if p.exists() else None


def get(x, path):
    for key in path:
        x = x[key]
    return x


families = {}
for family in ['diagnostics', 'source', 'chunk-contracts', 'source-effects', 'contracts']:
    a, b = [load(side, family) for side in ['before', 'after']]
    assert (a is None) == (b is None), f'{family}: presence changed'
    families[family] = dict(present=a is not None, changes=diff(norm(a), norm(b)))
(OUT / 'precise-field-differences.json.gz').write_bytes(gzip.compress(
    (json.dumps(families, indent=2) + '\n').encode(), mtime=0))
a, b = [load(side, 'contracts') for side in ['before', 'after']]
key = 'sources' if 'sources' in a else 'contracts'
raw_a, raw_b = a[key], b[key]
na, nb = norm(raw_a), norm(raw_b)
texts = {k: list(difflib.unified_diff(na[k].splitlines(), nb[k].splitlines(),
                                  fromfile='before', tofile='after'))
         for k in na.keys() & nb.keys() if na[k] != nb[k]}
keys = dict(meaning='Raw keys and aliases are retained; comparison replaces only the explicit checkout prefixes. No key is dropped.',
            field=key, before_raw_keys=sorted(raw_a), after_raw_keys=sorted(raw_b),
            raw_removed=sorted(raw_a.keys()-raw_b.keys()), raw_added=sorted(raw_b.keys()-raw_a.keys()),
            before_normalized_keys=sorted(na), after_normalized_keys=sorted(nb),
            normalized_removed=sorted(na.keys()-nb.keys()), normalized_added=sorted(nb.keys()-na.keys()),
            removed_module_texts={k:na[k] for k in na.keys()-nb.keys()},
            added_module_texts={k:nb[k] for k in nb.keys()-na.keys()},
            normalized_key_sets_equal=na.keys()==nb.keys(), changed_texts=texts)
(OUT / 'contract-key-review.json').write_text(json.dumps(keys, indent=2)+'\n')

# These finite, published mappings are limited to source-effect identity fields.
# Complete payload equality afterwards is required; program/assumption/package
# or decision changes cannot be hidden by deleting fields or broad regexes.
ea, eb = [norm(load(side, 'source-effects')) for side in ['before', 'after']]
identities = []
assert (ea is None) == (eb is None), 'source-effect presence changed'
if ea is not None:
    assert len(ea['results']) == len(eb['results'])
for i, (x, y) in enumerate(zip((ea or {}).get('results', []), (eb or {}).get('results', []))):
    if x == y:
        continue
    mapping = {}
    def add(old, new):
        if old != new:
            assert isinstance(old, str) and isinstance(new, str)
            assert old not in mapping or mapping[old] == new
            mapping[old] = new
    for p in [('evidence','unit','build_digest'), ('import_identity','configuration_digest'),
              ('task','unit','build_digest'), ('task','contract_sources','qcore:boolean-field-import:v1'),
              ('witness','task_digest')]:
        add(get(x,p), get(y,p))
    assert x['import_identity']['helpers'].keys() == y['import_identity']['helpers'].keys()
    for h in x['import_identity']['helpers']:
        add(x['import_identity']['helpers'][h]['configuration_digest'],
            y['import_identity']['helpers'][h]['configuration_digest'])
    assert len(x['bounds']) == len(y['bounds']) == 1
    add(next(iter(x['bounds'])), next(iter(y['bounds'])))
    assert norm(x, mapping) == y, 'unexplained source-effect payload change'
    identities.append(dict(index=i, library=x['library'], file=x['file'], function=x['function'],
                           exact_mapping=mapping, complete_payload_equal_after_mapping=True))
(OUT / 'source-effect-identity-review.json').write_text(json.dumps(dict(
    meaning='Finite build/configuration/obligation/witness identity correspondence; raw artifacts and all differences retained.',
    reviewed=identities), indent=2)+'\n')
print(json.dumps(dict(route=sys.argv[1], precise_differences={k:len(v['changes']) for k,v in families.items()},
                      contract_keys=len(na), changed_module_texts=list(texts),
                      source_effect_identity_rows=len(identities)), indent=2))
