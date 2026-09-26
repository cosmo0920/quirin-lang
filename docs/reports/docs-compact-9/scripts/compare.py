"""Compare two inventories: rules, invariants, anchors, names, pointers, links."""
import json
import re
import sys
from pathlib import Path

a = json.loads(Path(sys.argv[1]).read_text())
b = json.loads(Path(sys.argv[2]).read_text())
checkpoints = ''.join(Path(p).read_text() for p in sys.argv[3:])
out = []
ra, rb = a['rules'], b['rules']
out.append(f"rule blocks: {len(ra)} -> {len(rb)}; ids equal: {sorted(ra) == sorted(rb)}")
labels_equal = all(ra[k]['label'] == rb[k]['label'] and ra[k]['file'] == rb[k]['file'] for k in ra if k in rb)
out.append(f"labels and files unchanged: {labels_equal}")
same = [k for k in ra if k in rb and ra[k]['sha256'] == rb[k]['sha256']]
moved = [k for k in ra if k in rb and ra[k]['sha256'] != rb[k]['sha256']]
out.append(f"byte-identical rule blocks: {len(same)} of {len(ra)}; changed: {', '.join(moved)}")
for k in moved:
    lost_names = sorted(set(ra[k]['names']) - set(rb[k]['names']))
    lost_ptrs = sorted(set(ra[k]['pointers']) - set(rb[k]['pointers']))
    out.append(f"  {k} ({ra[k]['file']}): {ra[k]['bytes']} -> {rb[k]['bytes']} bytes; "
               f"pointers lost {lost_ptrs or 'none'}; names no longer in the rule {lost_names or 'none'}")
out.append(f"hard invariants: {len(a['invariants'])} -> {len(b['invariants'])}; identical: {a['invariants'] == b['invariants']}")
# Anchors: every base anchor of every base file must still exist.
lost = {f: sorted(set(v) - set(b['anchors'].get(f, []))) for f, v in a['anchors'].items()}
lost = {f: v for f, v in lost.items() if v}
out.append(f"anchors lost: {lost or 'none'}")
# Links: unresolved set before/after.
ua = {(l[0], l[1]) for l in a['links'] if not (l[3] and l[4])}
ub = {(l[0], l[1]) for l in b['links'] if not (l[3] and l[4])}
out.append(f"unresolved local links: {len(ua)} -> {len(ub)}; new: {sorted(ub - ua) or 'none'}; fixed: {sorted(ua - ub) or 'none'}")
# Links into changed files with a fragment: count resolved.
touched = {f for f in b['files'] if a['files'].get(f, {}).get('bytes') != b['files'][f]['bytes']}
frag = [l for l in b['links'] if '#' in l[1] and l[2] in touched | {'docs/development/increments/README.md'}]
out.append(f"links with a fragment into changed rule/status pages: {len(frag)}, unresolved {sum(1 for l in frag if not (l[3] and l[4]))}")
# Names per file: lost names must appear in a checkpoint.
for f in sorted(b['files']):
    na, nb = set(a['files'][f]['names']), set(b['files'][f]['names'])
    gone = sorted(na - nb)
    if gone:
        missing = [n for n in gone if n not in checkpoints]
        out.append(f"{f}: {len(gone)} names no longer on the page; not in a checkpoint: {missing or 'none'}")
        out.append('    ' + ', '.join(gone))
    pa, pb = set(a['files'][f]['pointers']), set(b['files'][f]['pointers'])
    if pa - pb:
        out.append(f"{f}: report pointers no longer on the page: {sorted(pa - pb)}; in a checkpoint: "
                   f"{[p for p in sorted(pa - pb) if p in checkpoints]}")
for f in sorted(b['files']):
    if a['files'][f]['bytes'] != b['files'][f]['bytes']:
        out.append(f"bytes {f}: {a['files'][f]['bytes']} -> {b['files'][f]['bytes']}")
print('\n'.join(out))
