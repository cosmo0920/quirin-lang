"""Apply line-range replacements to one file at the base revision.

Usage: apply_ranges.py SPEC.json
SPEC: {"file": path, "base": rev, "edits": [{"start": n, "end": m, "with": path-or-null,
       "moved": [[a, b], ...], "title": str, "note": str}]}
`start..end` (1-based, inclusive) of the base file is replaced by the text of
`with` (or deleted when null). `moved` lists the base line ranges whose text
leaves the current authority; checkpoint.py copies exactly those ranges
verbatim. The spec is the record of what moved.
"""
import json
import subprocess
import sys
from pathlib import Path

spec = json.loads(Path(sys.argv[1]).read_text())
lines = subprocess.check_output(['git', 'show', f"{spec['base']}:{spec['file']}"]).decode().split('\n')
edits = sorted(spec['edits'], key=lambda e: e['start'])
for a, b in zip(edits, edits[1:]):
    assert a['end'] < b['start'], (a, b)
out = []
i = 1
while i <= len(lines):
    e = next((e for e in edits if e['start'] == i), None)
    if e:
        if e['with']:
            out.append(Path(e['with']).read_text().rstrip('\n'))
        i = e['end'] + 1
    else:
        out.append(lines[i - 1])
        i += 1
Path(spec['file']).write_text('\n'.join(out))
print(spec['file'], len('\n'.join(out).encode()), 'bytes')
