#!/usr/bin/env python3
"""Measure one frozen W48-BU-SINGLE-LEAF-BODIES snapshot on a fresh private route.

Usage: measure.py ROUTE OUT --link-map MAP --inputs-snapshot SNAPSHOT --binary-source REV [--jobs N]

Adapted from docs/reports/w48-bottom-up-batch5/scripts/measure_strong.py: the
same environment (explicit link map, llvm-21 Clang for configured routes,
null and outer projections, no source contracts or census), with the
snapshot's own runner, profiles and release binary. Input identities are
recorded before and after; any change fails the route (exit 101).

Copied from docs/reports/w48-sched-cleanup-production/scripts/measure.py
(card/w48-sched-cleanup-production) so the corpus figures are comparable
across cards. Two changes: PRIMARY is the real checkout path (the old
qcore-c path is a symlink to it), and the slice route reads the tail-pause
preparation (`private/w47-tail/attempt-1/prep`) as batch5's measure_strong.py
does; the scheduler card never ran the slice route.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

PRIMARY = Path('/media/Data/GitHub/quirin-lang')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def tree_identity(root):
    return {str(p.relative_to(root)): sha(p) for p in sorted(root.rglob('*'))
            if p.is_file() and p.relative_to(root).parts[0] != 'target'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('route', choices=['corpus', 'system', 'slice'])
    parser.add_argument('out', type=Path)
    parser.add_argument('--link-map', type=Path, required=True)
    parser.add_argument('--inputs-snapshot', type=Path, required=True)
    parser.add_argument('--binary-source', required=True)
    parser.add_argument('--jobs', type=int, default=4)
    args = parser.parse_args()
    assert args.jobs > 0
    free = shutil.disk_usage(PRIMARY).free
    assert free >= 40 * 1024**3, f'require 40GiB free before measurement; available {free} bytes'
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    route = args.route
    snapshot = args.inputs_snapshot.resolve()
    binary = snapshot / 'target/release/quirin-audit'
    link_map = args.link_map.resolve()
    env = dict(os.environ)
    for key in ('QCORE_SOURCE_CONTRACTS', 'QCORE_RETURN_CENSUS', 'QCORE_CHUNKIO_PACKAGE'):
        env.pop(key, None)
    env['QCORE_PROFILES'] = str(snapshot / 'contract/profiles')
    env.update(QCORE_CROSS_UNIT_LINK_MAP=str(link_map), QCORE_CLANG='/usr/lib/llvm-21/bin/clang',
               QCORE_NULL_PROPERTY=str(out / f'{route}-null.jsonl'),
               QCORE_OUTER_PROJECTION=str(out / f'{route}-outer.json'))
    if route == 'corpus':
        command = [sys.executable, str(snapshot / 'scripts/run_pinned_audit.py'), str(out / 'corpus'),
                   '66910c10', '--inputs-root', str(PRIMARY / 'build/cycle19'), '--jobs', str(args.jobs)]
        externals = [PRIMARY / 'build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/build/compile_commands.json',
                     PRIMARY / 'build/cycle19/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json']
    else:
        prepared = PRIMARY / ('private/w45-fb/w46-external-callee-contracts/attempt-1' if route == 'system'
                              else 'private/w47-tail/attempt-1/prep')
        source = prepared / 'fluent-bit-clang-source'
        database = prepared / 'fluent-bit-clang-production/compile_commands.json'
        selection = snapshot / 'contract/profiles/fluent-bit' / ('system-selection.json' if route == 'system'
                                                          else 'tail-pause-selection.json')
        command = [str(binary), 'current', '--checkout', str(source), '--profile-directory',
                   str(snapshot / 'contract/profiles/fluent-bit'), '--compile-database', str(database),
                   '--selection', str(selection), '--jobs', str(args.jobs),
                   '--report', str(out / 'resource-diagnostics.json')]
        externals = [database, selection]
    record = {'analyzer_source': args.binary_source, 'inputs_snapshot': str(snapshot),
              'inputs_before': tree_identity(snapshot),
              'report_driver_sha256': sha(Path(__file__)),
              'binary': str(binary), 'binary_sha256': sha(binary), 'command': command,
              'jobs': args.jobs, 'link_map': str(link_map), 'link_map_sha256': sha(link_map),
              'externals_before': {str(p): sha(p) for p in externals},
              'environment': {k: env.get(k, 'unset') for k in (
                  'QCORE_PROFILES', 'QCORE_CLANG', 'QCORE_CROSS_UNIT_LINK_MAP', 'QCORE_NULL_PROPERTY',
                  'QCORE_OUTER_PROJECTION', 'QCORE_SOURCE_CONTRACTS', 'QCORE_RETURN_CENSUS',
                  'QCORE_CHUNKIO_PACKAGE', 'QCORE_AUDIT_JOBS')},
              'free_bytes_before': free,
              'started': datetime.datetime.now(datetime.timezone.utc).isoformat()}
    (out / 'invocation.json').write_text(json.dumps(record, indent=2) + '\n')
    with (out / 'console.log').open('w') as stdout, (out / 'stderr.log').open('w') as stderr:
        result = subprocess.run(command, cwd=snapshot, env=env, stdout=stdout, stderr=stderr)
    record.update(exit=result.returncode, ended=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  binary_sha256_after=sha(binary), free_bytes_after=shutil.disk_usage(PRIMARY).free,
                  inputs_after=tree_identity(snapshot), link_map_sha256_after=sha(link_map),
                  externals_after={str(p): sha(p) for p in externals})
    stable = (record['inputs_before'] == record['inputs_after']
              and record['externals_before'] == record['externals_after']
              and record['link_map_sha256'] == record['link_map_sha256_after']
              and record['binary_sha256'] == record['binary_sha256_after'])
    code = result.returncode
    if not stable:
        record['identity_error'] = 'an input or the binary changed during the route'
        record['audit_exit'] = code
        code = record['exit'] = 101
    (out / 'invocation.json').write_text(json.dumps(record, indent=2) + '\n')
    (out / f'{route}.exit').write_text(str(code) + '\n')
    return code


if __name__ == '__main__':
    raise SystemExit(main())
