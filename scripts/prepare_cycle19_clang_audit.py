#!/usr/bin/env python3
"""Prepare explicit Clang databases and optionally run the current Cycle19 audit.

The developer's GCC build is left intact. Source and generated headers live in
an independent revision-selected clone. No compiler command is rewritten.
"""
import argparse
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def capture(command):
    result = subprocess.run(command, text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip() or f'command failed: {command[0]}')
    return result.stdout.strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fluent-bit', type=Path, default=ROOT.parent / 'fluent-bit')
    parser.add_argument('--revision', default='HEAD')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--clang', default='clang')
    parser.add_argument('--clangxx', default='clang++')
    parser.add_argument('--run', action='store_true')
    args = parser.parse_args()
    clang, clangxx = shutil.which(args.clang), shutil.which(args.clangxx)
    if not clang or not clangxx:
        parser.error('Clang C and C++ compilers are required')
    upstream = args.fluent_bit.resolve(strict=True)
    revision = capture(['git', '-C', str(upstream), 'rev-parse', '--verify', '--end-of-options', f'{args.revision}^{{commit}}'])
    output = (args.output or ROOT / 'build/cycle19/clang' / revision).resolve()
    print(f'Preparing Clang audit for {revision} in {output}', file=sys.stderr, flush=True)
    config_path = capture([sys.executable, '-B', str(ROOT / 'scripts/prepare_cmetrics_clang_audit.py'),
                           '--fluent-bit', str(upstream), '--revision', revision, '--output', str(output),
                           '--clang', clang, '--clangxx', clangxx])
    config = json.loads(Path(config_path).read_text())
    source = Path(config['checkout'])
    monkey = capture([sys.executable, '-B', str(ROOT / 'scripts/prepare_cycle19_monkey.py'),
                      '--fluent-bit', str(source), '--output', str(output / 'monkey'),
                      '--clang', clang, '--clangxx', clangxx])
    reports = output / 'reports'
    reports.mkdir(parents=True, exist_ok=True)
    command = ['cargo', 'run', '--quiet', '-p', 'qcore-compiler',
               '--example', 'cycle19_fluent_bit_libraries_audit', '--',
               '--compile-database', config['compile_database'], str(source), monkey,
               str(reports / 'monkey-source.json'), str(reports / 'resource-diagnostics.json')]
    metadata = dict(source_revision=revision, clang=clang, clangxx=clangxx,
                    compile_database=config['compile_database'], monkey_database=monkey,
                    cwd=str(ROOT), command=command, environment={'QCORE_CLANG': clang},
                    scope='Explicit Clang build; cmetrics-static built, full native Fluent Bit binary not built; resource analysis does not establish GCC equivalence or whole-library safety.')
    (output / 'audit-command.json').write_text(json.dumps(metadata, indent=2) + '\n')
    print(shlex.join(['env', f'QCORE_CLANG={clang}'] + command), flush=True)
    if args.run:
        result = subprocess.run(command, cwd=ROOT, env=dict(os.environ, QCORE_CLANG=clang))
        raise SystemExit(result.returncode)


if __name__ == '__main__':
    main()
