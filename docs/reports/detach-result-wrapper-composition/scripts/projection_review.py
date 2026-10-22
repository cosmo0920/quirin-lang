#!/usr/bin/env python3
"""Null-property and outer-projection review for BASE/TIP snapshots.

Usage: projection_review.py BASE_ROUTE_DIR TIP_ROUTE_DIR OUT_JSON

Both sides are read with only the snapshot directory name rewritten
(`base-src` to `tip-src`) and every 64-hex digest masked, because separately
exported snapshots put their own provider paths into every environment digest.
Reports every null row whose record differs (with the null fields compared
apart from `custody`, `combined` and `hypothesis_context`), every differing
leaf of the null summary and of the outer projection. Read-only.
"""
import json
import pathlib
import re
import sys

HEX = re.compile(r'[0-9a-f]{64}')


def text(root, name):
    raw = (pathlib.Path(root) / name).read_text()
    return HEX.sub('<sha>', raw.replace('detach-wrapper/base-src/', 'detach-wrapper/tip-src/'))


def leaves(a, b, path, out):
    if isinstance(a, dict) and isinstance(b, dict):
        for key in sorted(set(a) | set(b)):
            if a.get(key) != b.get(key):
                leaves(a.get(key), b.get(key), f'{path}.{key}', out)
    elif isinstance(a, list) and isinstance(b, list) and len(a) == len(b):
        for index, (x, y) in enumerate(zip(a, b)):
            if x != y:
                leaves(x, y, f'{path}[{index}]', out)
    else:
        out.append(dict(path=path, base=a, tip=b))


def main():
    base_root, tip_root = sys.argv[1], sys.argv[2]
    null = {}
    for side, root in [('base', base_root), ('tip', tip_root)]:
        for line in text(root, 'corpus-null.jsonl').splitlines():
            record = json.loads(line)
            null.setdefault(f"{record['file']}::{record['function']}", {})[side] = record
    rows = []
    for key, sides in sorted(null.items()):
        if sides.get('base') != sides.get('tip'):
            b, t = sides.get('base') or {}, sides.get('tip') or {}
            ignore = {'custody', 'combined', 'hypothesis_context'}
            rows.append(dict(row=key, custody=[b.get('custody'), t.get('custody')],
                             combined=[b.get('combined'), t.get('combined')],
                             null_fields_equal={k: v for k, v in b.items() if k not in ignore}
                             == {k: v for k, v in t.items() if k not in ignore}))
    summary, outer = [], []
    leaves(json.loads(text(base_root, 'corpus-null.summary.json')),
           json.loads(text(tip_root, 'corpus-null.summary.json')), '', summary)
    leaves(json.loads(text(base_root, 'corpus-outer.json')),
           json.loads(text(tip_root, 'corpus-outer.json')), '', outer)
    out = dict(null_rows_total=len(null), null_rows_changed=rows, null_summary=summary, outer=outer)
    pathlib.Path(sys.argv[3]).write_text(json.dumps(out, indent=1) + '\n')
    print(json.dumps(dict(null_rows_total=len(null), null_rows_changed=rows,
                          null_summary=summary,
                          outer=[dict(path=o['path'], base=json.dumps(o['base'])[:200],
                                      tip=json.dumps(o['tip'])[:200]) for o in outer[:40]],
                          outer_count=len(outer)), indent=1))


if __name__ == '__main__':
    raise SystemExit(main())
