"""W48-OPENSSL-CONTRACT-PLUGIN: attribute the measured `review_holds`
content changes. For every function row whose `review_holds` differ between
the two sides' component sidecars, print both values with each side's
checkout root replaced by `<root>`; equal after that substitution means the
change is the checkout path only (environment).

usage: python3 -I review_holds_diff.py BASE.components.json TIP.components.json BASE_ROOT TIP_ROOT
"""
import json
import sys


def rows(path):
    found = {}

    def walk(node, key=None):
        if isinstance(node, dict):
            if 'review_holds' in node:
                found[json.dumps(node.get('component') or node.get('function') or key, sort_keys=True)] = node['review_holds']
            for k, v in node.items():
                walk(v, k)
        elif isinstance(node, list):
            for item in node:
                walk(item, key)
    walk(json.load(open(path)))
    return found


base, tip = rows(sys.argv[1]), rows(sys.argv[2])
base_root, tip_root = sys.argv[3], sys.argv[4]
changed = same_after = 0
for key in sorted(set(base) | set(tip)):
    b = json.dumps(base.get(key), sort_keys=True)
    t = json.dumps(tip.get(key), sort_keys=True)
    if b == t:
        continue
    changed += 1
    if b.replace(base_root, '<root>') == t.replace(tip_root, '<root>'):
        same_after += 1
    else:
        print('DIFFERS', key, b[:400], t[:400])
print('review_holds differing:', changed, 'equal after root substitution:', same_after)
