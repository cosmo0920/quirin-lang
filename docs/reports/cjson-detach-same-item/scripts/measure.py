#!/usr/bin/env python3
"""Measure the composed checked-detach source on a fresh private route with both projections."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

REPO = Path(__file__).resolve().parents[4]
PRIMARY = Path('/home/cosmo/Documents/GitHub/qcore-c')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('route', choices=['corpus', 'system', 'slice'])
    parser.add_argument('out', type=Path)
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    route = args.route
    binary = REPO / 'target/release/quirin-audit'
    link_map = REPO / 'docs/reports/w48-bottom-up-batch2/link-map.corpus.json'
    env = dict(os.environ)
    for key in ('QCORE_SOURCE_CONTRACTS', 'QCORE_RETURN_CENSUS'):
        env.pop(key, None)
    env.update(QCORE_CROSS_UNIT_LINK_MAP=str(link_map), QCORE_CLANG='/usr/lib/llvm-21/bin/clang',
               QCORE_NULL_PROPERTY=str(out / f'{route}-null.jsonl'),
               QCORE_OUTER_PROJECTION=str(out / f'{route}-outer.json'))
    if route == 'corpus':
        command = [sys.executable, str(REPO / 'scripts/run_pinned_audit.py'), str(out / 'corpus'),
                   '66910c10', '--inputs-root', str(PRIMARY / 'build/cycle19'), '--jobs', '4']
        source = PRIMARY / 'build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source'
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
    record = {'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip(),
              'binary': str(binary), 'binary_sha256': sha(binary), 'command': command, 'jobs': 4,
              'source_head': subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip(),
              'link_map': str(link_map), 'link_map_sha256': sha(link_map),
              'environment': {k: env.get(k, 'unset') for k in ('QCORE_CLANG', 'QCORE_CROSS_UNIT_LINK_MAP',
                              'QCORE_NULL_PROPERTY', 'QCORE_OUTER_PROJECTION', 'QCORE_SOURCE_CONTRACTS', 'QCORE_RETURN_CENSUS')},
              'started': datetime.datetime.now(datetime.timezone.utc).isoformat()}
    if route != 'corpus':
        record.update(compile_database=str(database), compile_database_sha256=sha(database),
                      selection=str(selection), selection_sha256=sha(selection))
    (out / 'invocation.json').write_text(json.dumps(record, indent=2) + '\n')
    with (out / 'console.log').open('w') as stdout, (out / 'stderr.log').open('w') as stderr:
        result = subprocess.run(command, cwd=REPO, env=env, stdout=stdout, stderr=stderr)
    record.update(exit=result.returncode, ended=datetime.datetime.now(datetime.timezone.utc).isoformat())
    (out / 'invocation.json').write_text(json.dumps(record, indent=2) + '\n')
    (out / f'{route}.exit').write_text(str(result.returncode) + '\n')
    return result.returncode


if __name__ == '__main__':
    raise SystemExit(main())
