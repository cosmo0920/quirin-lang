#!/usr/bin/env python3
"""Keep declaration authority and usage counts distinct in each fresh route."""
import gzip
import json
from pathlib import Path
import sys

REPO = Path(__file__).resolve().parents[4]
REPORT = Path(__file__).resolve().parents[1]


def normalize(value):
    if isinstance(value, str):
        for name in ['inputs-base', 'inputs-tip']:
            value = value.replace(str(REPO / 'private/w48-bottom-up-batch4' / name), '$INPUTS')
        return value
    if isinstance(value, list):
        return [normalize(x) for x in value]
    if isinstance(value, dict):
        result = {normalize(k): normalize(v) for k, v in value.items()}
        assert len(result) == len(value), 'normalization collision'
        return result
    return value


route = sys.argv[1]
out = REPORT / 'artifacts' / route
left, right = [json.loads(gzip.decompress((out / side / 'contracts.json.gz').read_bytes()))
               for side in ['before', 'after']]
a, b = [normalize(d['contract_dependencies']['declarations']) for d in [left, right]]
assert a.keys() == b.keys(), 'declaration inventory changed; inspect before extending this review'
changes = []
body_occurrences = []
for name in a:
    changed = [k for k in sorted(a[name].keys() | b[name].keys()) if a[name].get(k) != b[name].get(k)]
    authority = [k for k in changed if k not in ['rows', 'safe_rows', 'bodies']]
    if 'bodies' in changed:
        old, new = a[name]['bodies'], b[name]['bodies']
        assert old.keys() == new.keys()
        fields = [k for k in old if old[k] != new[k]]
        assert set(fields) <= {'validated_units', 'unvalidated_units'}, (name, fields)
        body_occurrences.append(dict(function=name, changed_fields=fields, before=old, after=new,
                                     meaning='Caller-session body replays, not distinct native C definitions; standing and proof methods unchanged.'))
    assert not authority, (name, authority)
    if changed:
        changes.append(dict(function=name, changed_fields=changed, before=a[name], after=b[name]))
result = dict(route=route, declarations=len(a), changed=changes,
              inventory_equal=True, body_replay_occurrence_changes=body_occurrences, unchanged_authority=True,
              errno_remains_declared_assumption=True,
              before_counts=left['contract_dependencies']['counts'],
              after_counts=right['contract_dependencies']['counts'],
              before_safe_rollup=left['contract_dependencies']['safe_rollup'],
              after_safe_rollup=right['contract_dependencies']['safe_rollup'])
(out / 'authority-review.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(dict(route=route, declarations=len(a), changed=[x['function'] for x in changes])))
