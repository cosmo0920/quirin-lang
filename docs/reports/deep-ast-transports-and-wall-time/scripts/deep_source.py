#!/usr/bin/env python3
"""deep_source.py UNIT-SUFFIX OUT.i [CLANG] -- DEEP-AST-TRANSPORTS-AND-WALL-TIME
(adapted from RDHTTP-HEADER-SUMMARY-GAP's scripts/depth.py): preprocess one unit
of the pinned 66910c10 compile database with its own flags (`-E`, read only
from build/cycle19, written to OUT.i), for the measurement-only test
`deep_ast_transports::tests::measure_the_readers_on_a_preprocessed_unit`, and
print the JSON nesting of its whole-unit AST as serde_json counts it."""
import json
import shlex
import subprocess
import sys

BASE = ('/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/clang/'
        '66910c10a4d7eafa810b84229cf2dcf7b0f26f97/fluent-bit-clang-production')


def flags(entry):
    args = shlex.split(entry['command'])
    kept, skip = [], False
    for arg in args[1:]:
        if skip:
            skip = False
            continue
        if arg in ('-o', '-c'):
            skip = True
            continue
        if arg.startswith('-Wa,') or arg == entry['file']:
            continue
        kept.append(arg)
    return kept


def depth(data):
    level = deepest = 0
    in_str = esc = False
    for c in data:
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
            level += 1
            deepest = max(deepest, level)
        elif c in (0x5d, 0x7d):
            level -= 1
    return deepest


def main():
    suffix, out = sys.argv[1], sys.argv[2]
    clang = sys.argv[3] if len(sys.argv) > 3 else '/usr/bin/clang'
    database = json.load(open(BASE + '/compile_commands.json'))
    [entry] = [e for e in database if e['file'].endswith(suffix)]
    pre = subprocess.run([clang] + flags(entry) + ['-E', entry['file'], '-o', out],
                         cwd=entry['directory'])
    print('preprocess exit', pre.returncode)
    text = open(out, 'rb').read()
    ast = subprocess.run([clang, '-x', 'c', '-std=gnu17', '-fsyntax-only', '-Xclang', '-ast-dump=json', '-'],
                         input=text, capture_output=True)
    print('ast exit', ast.returncode, 'bytes', len(ast.stdout), 'depth', depth(ast.stdout))


if __name__ == '__main__':
    main()
