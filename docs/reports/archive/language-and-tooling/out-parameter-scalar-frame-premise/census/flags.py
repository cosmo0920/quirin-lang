"""Census of strict-aliasing flags in compile databases.

usage: flags.py <out.json> <compile_commands.json>...
"""
import json, re, shlex, sys, collections as C

FLAGS = ('-fno-strict-aliasing', '-fstrict-aliasing', '-Wstrict-aliasing', '-O0', '-O1', '-O2',
         '-O3', '-Os', '-Oz', '-Og')
out = {}
for path in sys.argv[2:]:
    entries = json.load(open(path))
    per_lib = C.defaultdict(C.Counter)
    files = {}
    for e in entries:
        args = e.get('arguments') or shlex.split(e.get('command', ''))
        f = e['file']
        m = re.search(r'/(lib/[^/]+|src|plugins/[^/]+|mk_server|mk_core)/', f)
        lib = m.group(1) if m else 'other'
        seen = sorted({a for a in args if a in FLAGS or a.startswith('-fno-strict') or a.startswith('-fstrict')})
        per_lib[lib][' '.join(seen) or '(none)'] += 1
        files[f] = seen
    out[path] = dict(entries=len(entries),
                     by_library={k: dict(v) for k, v in sorted(per_lib.items())},
                     files=files)
    print(path, len(entries))
    for lib, counts in sorted(per_lib.items()):
        print('  ', lib, dict(counts))
json.dump(out, open(sys.argv[1], 'w'), indent=1)
