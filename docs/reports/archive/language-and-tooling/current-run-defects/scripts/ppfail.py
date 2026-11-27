"""Which units of a compile database fail `-E` under their own command."""
import json, shlex, subprocess, sys, os
from concurrent.futures import ThreadPoolExecutor

db = json.load(open(sys.argv[1]))
pattern = sys.argv[2] if len(sys.argv) > 2 else ''


def own(entry):
    words = shlex.split(entry['command'])
    out = [words[0]]
    skip = False
    for w in words[1:]:
        if skip:
            skip = False
            continue
        if w in ('-c', '-MD', '-MMD', '-MP', '-M', '-MM', '-MG'):
            continue
        if w in ('-o', '-MF', '-MT', '-MQ'):
            skip = True
            continue
        if w.startswith('-o') and len(w) > 2:
            continue
        out.append(w)
    return out + ['-E', '-o', '/dev/null']


def run(entry):
    p = subprocess.run(own(entry), cwd=entry['directory'], capture_output=True, text=True)
    first = p.stderr.splitlines()[0] if p.stderr else ''
    return entry['file'], p.returncode, first


entries = [e for e in db if pattern in e['file']]
with ThreadPoolExecutor(6) as pool:
    results = list(pool.map(run, entries))
fails = [r for r in results if r[1] != 0]
print('units', len(entries), 'failed', len(fails))
for f, code, first in sorted(fails):
    print(f.split('fluent-bit-clang-source/', 1)[-1] if 'fluent-bit-clang-source/' in f else f, '|', first[:200])
