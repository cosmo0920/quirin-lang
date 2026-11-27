"""Compare the census driver's verdicts without (OFF) and with (ON) rule 41.

Usage: compare.py OFF_DIR ON_DIR OUT.json

Per function: status before/after, the five families' leaves removed, and
every diagnostic key that appears only with the rule (a surfaced finding).
"""
import collections as C, json, os, sys

FAMILIES = ['unsupported assignment expression', 'unrecognized assignment destination',
            'braced initializer, expression, or declaration is not modeled',
            'unmodeled increment or decrement expression',
            'C operator `sizeof` requires type/operand analysis']
off_dir, on_dir, out = sys.argv[1:4]
moves = C.Counter()
removed = C.Counter()
rows = []
for name in sorted(os.listdir(off_dir)):
    if not name.endswith('.verdicts.json'):
        continue
    unit = name[:-len('.verdicts.json')].replace('__', '/')
    try:
        a = json.load(open(os.path.join(off_dir, name)))
        b = json.load(open(os.path.join(on_dir, name)))
    except FileNotFoundError:
        rows.append({'unit': unit, 'missing': True})
        continue
    for fn in sorted(set(a) | set(b)):
        sa, ka = a.get(fn, ['absent', []])
        sb, kb = b.get(fn, ['absent', []])
        fam = lambda keys: [k for k in keys if any(f in k for f in FAMILIES)]
        gone = len(fam(ka)) - len(fam(kb))
        new = sorted(set(kb) - set(ka))
        lost = sorted(set(ka) - set(kb))
        nonfam_new = [k for k in new if not any(f in k for f in FAMILIES)]
        nonfam_lost = [k for k in lost if not any(f in k for f in FAMILIES)]
        if sa != sb or gone or nonfam_new or nonfam_lost:
            moves[(sa, sb)] += 1
            removed['family leaves removed'] += gone
            rows.append({'unit': unit, 'function': fn, 'off': sa, 'on': sb, 'family_leaves_removed': gone,
                         'new_other': nonfam_new, 'lost_other': nonfam_lost})
json.dump(rows, open(out, 'w'), indent=1)
print(dict(moves))
print(dict(removed))
for r in rows:
    if r.get('off') != r.get('on') or r.get('new_other') or r.get('lost_other'):
        print(r)
