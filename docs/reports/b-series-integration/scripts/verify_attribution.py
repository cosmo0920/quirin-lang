#!/usr/bin/env python3
"""Check the archived final coordinate-multiset attribution to isolated cards."""
import collections
import json
from pathlib import Path

artifacts = Path(__file__).resolve().parents[1] / 'artifacts'
combined = json.loads((artifacts / 'delta.json').read_text())
parts = {name: json.loads((artifacts / f'{name}-isolated-delta.json').read_text())
         for name in ['b01', 'b02', 'b03', 'b04']}

def bag(delta, field):
    result = collections.Counter()
    for item in delta[field]:
        result[tuple(item['key'])] += item['count']
    return result

for field in ['leaves_removed', 'leaves_added']:
    assert bag(combined, field) == sum((bag(d, field) for d in parts.values()), collections.Counter()), field
assert not combined['unit_moves']
assert all(not d['unit_moves'] for d in parts.values())
assert sorted(json.dumps(x, sort_keys=True) for x in combined['function_moves']) == sorted(
    json.dumps(x, sort_keys=True) for d in parts.values() for x in d['function_moves'])
assert not combined['leaves_added']
print('PASS: final diagnostic multiset delta equals the sum of the four isolated deltas.')
print('PASS: final function moves equal their disjoint union; no unit moves or added leaves.')
for name, delta in parts.items():
    print(f"{name}: {sum(bag(delta, 'leaves_removed').values())} removed leaves; {len(delta['function_moves'])} function moves")
