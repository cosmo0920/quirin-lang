#!/usr/bin/env python3
"""Census of a compile database's non-C units against the pre-pass tokens.

Card CALLBACK-MEMBER-NON-C-UNITS. Read-only: for every unit of `--database`
whose file is not `.c`, preprocess it with its own recorded command (`-c`,
`-o FILE` and dependency flags dropped, `-E` added, run from its directory,
as `member_prepass::preprocess_own` does) and list the pre-pass tokens
(`--sidecar`'s `callback_members.prepass.tokens`) its text names. With
`--ast`, also dump the C++ units' Clang AST (`-fsyntax-only -Xclang
-ast-dump`) and count, per named token, the node kinds whose dump line
names it: evidence for the follow-up on a clang++ store reading, not used
by the analyzer.
"""
import argparse
import builtins
import collections
import json
import re
import shlex
import subprocess


def own_arguments(entry):
    words = shlex.split(entry['command']) if 'command' in entry else list(entry['arguments'])
    out, skip = [], False
    for word in words[1:]:
        if skip:
            skip = False
            continue
        if word in ('-c', '-MD', '-MMD', '-MP', '-M', '-MM', '-MG'):
            continue
        if word in ('-o', '-MF', '-MT', '-MQ'):
            skip = True
            continue
        out.append(word)
    return words[0], out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--database', required=True)
    parser.add_argument('--sidecar', required=True, help='a run\'s resource-diagnostics.contracts.json')
    parser.add_argument('--root', default='', help='a path prefix to strip from printed paths')
    parser.add_argument('--ast', action='store_true')
    parser.add_argument('--only', action='append', default=[],
                        help='dump the AST only of units whose path contains this (repeatable)')
    parser.add_argument('--out', help='write the census here instead of standard output')
    args = parser.parse_args()
    sink = open(args.out, 'w') if args.out else None

    def print(*items):
        builtins.print(*items, file=sink)

    tokens = json.load(open(args.sidecar))['callback_members']['prepass']['tokens']
    database = json.load(open(args.database))
    for entry in database:
        path = entry['file']
        if path.endswith('.c'):
            continue
        shown = path.replace(args.root, '')
        program, rest = own_arguments(entry)
        run = subprocess.run([program, *rest, '-E'], cwd=entry['directory'],
                             capture_output=True, text=True)
        if run.returncode != 0:
            print(f'{shown}: preprocessing failed: {run.stderr.splitlines()[:1]}')
            continue
        words = set(re.findall(r'[A-Za-z_][A-Za-z_0-9]*', run.stdout))
        named = [token for token in tokens if token in words]
        print(f'{shown}: {len(run.stdout)} bytes, names {named}')
        if not (args.ast and named and path.endswith(('.cpp', '.cc', '.cxx'))):
            continue
        if args.only and not any(fragment in path for fragment in args.only):
            continue
        dump = subprocess.Popen([program, *rest, '-fsyntax-only', '-Xclang', '-ast-dump',
                                 '-fno-color-diagnostics'], cwd=entry['directory'],
                                stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        kinds = collections.Counter()
        for line in dump.stdout:
            for token in named:
                if re.search(r'\b' + re.escape(token) + r'\b', line):
                    kind = re.match(r'^[ |`\-]*([A-Za-z]+)', line)
                    kinds[(token, kind.group(1) if kind else '?')] += 1
        dump.wait()
        print(f'  ast exit {dump.returncode}')
        for (token, kind), count in sorted(kinds.items()):
            print(f'  {token}: {kind} {count}')


if __name__ == '__main__':
    main()
