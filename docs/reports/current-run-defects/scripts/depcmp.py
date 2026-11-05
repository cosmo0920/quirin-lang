"""Compare each unit's preprocessed text (-E -P) between two prepared builds
of one revision, paths normalised: which units the generated inputs change."""
import json, shlex, subprocess, sys, hashlib
from concurrent.futures import ThreadPoolExecutor

dba, roota, dbb, rootb = sys.argv[1:5]


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
    return out + ['-E', '-P']


def run(args):
    entry, root = args
    p = subprocess.run(own(entry), cwd=entry['directory'], capture_output=True)
    if p.returncode != 0:
        return None
    return hashlib.sha256(p.stdout.replace(root.encode(), b'R/')).hexdigest()


def key(entry, root):
    return entry['file'].replace(root, 'R/')


a = {key(e, roota): e for e in json.load(open(dba))}
b = {key(e, rootb): e for e in json.load(open(dbb))}
names = sorted(set(a) & set(b))
with ThreadPoolExecutor(6) as pool:
    ha = list(pool.map(run, [(a[n], roota) for n in names]))
    hb = list(pool.map(run, [(b[n], rootb) for n in names]))
same = differ = newly = lost = 0
for n, x, y in zip(names, ha, hb):
    if x is None and y is None:
        continue
    if x is None:
        newly += 1
        print('NEWLY', n)
    elif y is None:
        lost += 1
        print('LOST', n)
    elif x == y:
        same += 1
    else:
        differ += 1
        print('DIFFER', n)
print('units', len(names), 'same', same, 'differ', differ, 'newly', newly, 'lost', lost)
