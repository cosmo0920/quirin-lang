#!/usr/bin/env python3
"""Run one frozen analyzer on the unchanged corpus/system/slice inputs."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess

REPO = Path(__file__).resolve().parents[4]
PRIMARY = Path('/home/cosmo/Documents/GitHub/qcore-c')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('route', choices=['corpus', 'system', 'slice'])
    parser.add_argument('binary', type=Path)
    parser.add_argument('analyzer_commit')
    parser.add_argument('out', type=Path)
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    route = args.route
    link_map = REPO / 'docs/reports/w48-bottom-up-batch2/link-map.corpus.json'
    env = dict(os.environ)
    for key in ('QCORE_SOURCE_CONTRACTS', 'QCORE_RETURN_CENSUS'):
        env.pop(key, None)
    env.update(QCORE_CROSS_UNIT_LINK_MAP=str(link_map), QCORE_CLANG='/usr/lib/llvm-21/bin/clang',
               QCORE_NULL_PROPERTY=str(out / f'{route}-null.jsonl'),
               QCORE_OUTER_PROJECTION=str(out / f'{route}-outer.json'))
    if route == 'corpus':
        source = PRIMARY / 'build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source'
        monkey = PRIMARY / 'build/cycle19/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json'
        env['QCORE_CHUNKIO_PACKAGE'] = str(PRIMARY / 'build/cycle19/provider-packages-wave20/qcore.chunkio')
        command = [str(binary), 'pinned', '--jobs', '4', str(source), str(monkey),
                   str(out / 'monkey-source.json'), str(out / 'resource-diagnostics.json')]
    else:
        prepared = PRIMARY / ('private/w45-fb/w46-external-callee-contracts/attempt-1' if route == 'system'
                              else 'private/w47-tail/attempt-1/prep')
        source = prepared / 'fluent-bit-clang-source'
        database = prepared / 'fluent-bit-clang-production/compile_commands.json'
        selection = REPO / 'contract/profiles/fluent-bit' / ('system-selection.json' if route == 'system'
                                                          else 'tail-pause-selection.json')
        command = [str(binary), 'current', '--checkout', str(source), '--profile-directory',
                   str(REPO / 'contract/profiles/fluent-bit'), '--compile-database', str(database),
                   '--selection', str(selection), '--jobs', '4', '--report', str(out / 'resource-diagnostics.json')]
    record = {'analyzer_source_commit': args.analyzer_commit, 'command': command, 'jobs': 4,
              'binary': str(binary), 'binary_sha256': sha(binary),
              'source_head': subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip(),
              'link_map': str(link_map), 'link_map_sha256': sha(link_map),
              'environment': {k: env.get(k, 'unset') for k in ('QCORE_CLANG', 'QCORE_CHUNKIO_PACKAGE', 'QCORE_CROSS_UNIT_LINK_MAP',
                              'QCORE_NULL_PROPERTY', 'QCORE_OUTER_PROJECTION', 'QCORE_SOURCE_CONTRACTS', 'QCORE_RETURN_CENSUS')},
              'started': datetime.datetime.now(datetime.timezone.utc).isoformat()}
    if route != 'corpus':
        record.update(compile_database=str(database), compile_database_sha256=sha(database),
                      selection=str(selection), selection_sha256=sha(selection))
    (out / 'invocation.json').write_text(json.dumps(record, indent=2) + '\n')
    with (out / 'console.log').open('wb') as stdout, (out / 'stderr.log').open('wb') as stderr:
        code = subprocess.run(command, cwd=REPO, env=env, stdout=stdout, stderr=stderr).returncode
    record.update(exit=code, ended=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  binary_sha256_after=sha(binary))
    if record['binary_sha256_after'] != record['binary_sha256']:
        record['audit_exit'] = code
        code = record['exit'] = 101
        record['identity_error'] = 'binary changed during route'
    (out / 'invocation.json').write_text(json.dumps(record, indent=2) + '\n')
    (out / 'exit').write_text(str(code) + '\n')
    return code


if __name__ == '__main__':
    raise SystemExit(main())
