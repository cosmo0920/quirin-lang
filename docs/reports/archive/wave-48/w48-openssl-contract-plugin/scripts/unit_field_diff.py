"""W48-OPENSSL-CONTRACT-PLUGIN: attribute the measured unit-field
differences. For every unit field the native comparison names as differing,
compare the two sides' values in the component sidecars after replacing each
side's checkout root with `<root>`; print the fields that still differ.

usage: python3 -I unit_field_diff.py NATIVE-COMPARISON.json ROUTE BASE.components.json TIP.components.json BASE_ROOT TIP_ROOT
"""
import json
import sys

record, route, base_path, tip_path, base_root, tip_root = sys.argv[1:7]
differences = json.load(open(record))['comparisons'][0]['routes'][route]['unit_field_differences']


def units(path):
    found = {}
    for node in json.load(open(path))['contribution']['components']:
        unit = node['unit']
        key = '::'.join([unit['library'], unit['file'], node['row'], node['property'], node['profile']])
        if key in differences:
            found[key] = dict(node.get('detail') or {}, **{k: v for k, v in node.items() if k != 'detail'})
    return found


base, tip = units(base_path), units(tip_path)
checked = residual = 0
for key, fields in differences.items():
    for field in fields:
        if key not in base or key not in tip:
            print('UNFOUND', key)
            residual += 1
            continue
        checked += 1
        b = json.dumps(base[key].get(field), sort_keys=True).replace(base_root, '<root>')
        t = json.dumps(tip[key].get(field), sort_keys=True).replace(tip_root, '<root>')
        if b != t:
            residual += 1
            print('DIFFERS', key, field, b[:300], '|', t[:300])
print('unit fields compared:', checked, 'still differing after root substitution:', residual)
