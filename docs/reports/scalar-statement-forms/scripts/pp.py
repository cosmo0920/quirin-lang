"""Preprocess each unit carrying a target leaf with clang -E -P (the gateway's
line coordinates), and attach the preprocessed statement text to every row."""
import json, os, shlex, subprocess, sys

B = '/media/Data/GitHub/quirin-lang/build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/'
CHECKOUT = B + 'fluent-bit-clang-source/'
DBS = [B + 'fluent-bit-clang-production/compile_commands.json', B + 'monkey/monkey-build/compile_commands.json']
OUT = sys.argv[2]
os.makedirs(OUT, exist_ok=True)
rows = json.load(open(sys.argv[1]))
cmds = {}
for db in DBS:
    try:
        for e in json.load(open(db)):
            f = os.path.normpath(os.path.join(e['directory'], e['file']))
            cmds.setdefault(f, e)
    except FileNotFoundError:
        pass


def args_of(e):
    a = e.get('arguments') or shlex.split(e['command'])
    keep = []
    i = 1
    std = '-std=gnu11'
    while i < len(a):
        x = a[i]
        if x in ('-I', '-D', '-include', '-isystem', '-U', '-imacros', '-iquote'):
            v = a[i + 1]
            if x in ('-I', '-isystem', '-iquote', '-include', '-imacros') and not os.path.isabs(v):
                v = os.path.join(e['directory'], v)
            keep += [x, v]
            i += 2
            continue
        if x.startswith(('-I', '-D', '-U', '-isystem')):
            if x.startswith('-I') and not os.path.isabs(x[2:]):
                x = '-I' + os.path.join(e['directory'], x[2:])
            keep.append(x)
        elif x.startswith('-std='):
            std = x
        i += 1
    return std, keep


done = {}
for u in sorted({r['unit'] for r in rows}):
    src = os.path.normpath(CHECKOUT + u)
    e = cmds.get(src)
    if not e:
        print('no command', u)
        continue
    std, keep = args_of(e)
    out = os.path.join(OUT, u.replace('/', '__') + '.i')
    p = subprocess.run(['/usr/lib/llvm-21/bin/clang', '-x', 'c', std, '-E', '-P', *keep, src, '-o', out],
                       cwd=e['directory'], capture_output=True, text=True)
    if p.returncode:
        print('fail', u, p.stderr[:300])
        continue
    done[u] = open(out, errors='replace').read().split('\n')
print('preprocessed', len(done))
json.dump({u: len(v) for u, v in done.items()}, open(os.path.join(OUT, 'lines.json'), 'w'))
