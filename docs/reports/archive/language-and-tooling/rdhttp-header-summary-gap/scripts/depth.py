"""depth.py UNIT-SUFFIX [CLANG] -- RDHTTP-HEADER-SUMMARY-GAP: run Clang's JSON AST
dump with the unit's own compile flags (the pinned 66910c10 compile database)
and report, per top-level declaration deeper than 100, its JSON nesting depth
counted from the document root exactly as serde_json counts it (each `[` or
`{` one level), with its kind, name and line. serde_json's default recursion
limit is 128."""
import json
import re
import shlex
import subprocess
import sys

BASE = ('/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/clang/'
        '66910c10a4d7eafa810b84229cf2dcf7b0f26f97/fluent-bit-clang-production')


def command(suffix, clang):
    database = json.load(open(BASE + '/compile_commands.json'))
    [entry] = [e for e in database if e['file'].endswith(suffix)]
    args = shlex.split(entry['command'])
    kept, skip = [], False
    for arg in args[1:]:
        if skip:
            skip = False
            continue
        if arg in ('-o', '-c'):
            skip = True
            continue
        if arg.startswith('-Wa,'):
            continue
        kept.append(arg)
    return entry, [clang] + kept + ['-fsyntax-only', '-Xclang', '-ast-dump=json', entry['file']]


def scan(data):
    depth = top_max = maxd = 0
    top_start = None
    in_str = esc = False
    found = []
    for i, c in enumerate(data):
        if in_str:
            if esc:
                esc = False
            elif c == 0x5c:
                esc = True
            elif c == 0x22:
                in_str = False
        elif c == 0x22:
            in_str = True
        elif c in (0x5b, 0x7b):
            depth += 1
            if depth == 3:
                top_start, top_max = i, 3
            top_max = max(top_max, depth)
            maxd = max(maxd, depth)
        elif c in (0x5d, 0x7d):
            if depth == 3 and top_max > 100:
                found.append((top_max, data[top_start:top_start + 4000].decode('utf-8', 'replace')))
            depth -= 1
    return maxd, found


def main():
    suffix = sys.argv[1]
    clang = sys.argv[2] if len(sys.argv) > 2 else 'clang'
    entry, cmd = command(suffix, clang)
    proc = subprocess.run(cmd, cwd=entry['directory'], capture_output=True)
    print('clang exit', proc.returncode, 'bytes', len(proc.stdout))
    maxd, found = scan(proc.stdout)
    print('max depth', maxd)
    for depth, head in found:
        kind = re.search(r'"kind": *"([^"]*)"', head)
        name = re.search(r'"name": *"([^"]*)"', head)
        line = re.search(r'"line": *(\d+)', head)
        print(depth, kind and kind.group(1), name and name.group(1), line and line.group(1))


if __name__ == '__main__':
    main()
