#!/usr/bin/env python3
"""Measure a configured route against an immutable binary and profile snapshot.

Run under the shared card validation lock. The working checkout can advance;
its HEAD is disclosed, while binary and inputs must remain byte-identical.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

PRIMARY = Path('/media/Data/GitHub/quirin-lang')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def source_identity(path):
    return {
        'head': subprocess.check_output(['git', '-C', str(path), 'rev-parse', 'HEAD'], text=True).strip(),
        'tracked_delta_sha256': hashlib.sha256(subprocess.check_output(
            ['git', '-C', str(path), 'diff', '--no-ext-diff', '--binary', 'HEAD'])).hexdigest(),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('route', choices=['system', 'slice'])
    parser.add_argument('out', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--snapshot', type=Path, required=True)
    parser.add_argument('--semantic-source', required=True)
    parser.add_argument('--link-map', type=Path, required=True)
    parser.add_argument('--primary-root', type=Path, default=PRIMARY)
    args = parser.parse_args()
    root = Path.cwd()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    snapshot = args.snapshot.resolve()
    binary = args.binary.resolve()
    link_map = args.link_map.resolve()
    manifest = json.loads((snapshot / 'manifest.json').read_text())
    def inputs():
        return {name: sha(snapshot / name) for name in manifest}
    before = inputs()
    if before != manifest:
        raise RuntimeError('frozen input manifest mismatch')
    if shutil.disk_usage(root).free < 40 * 1024**3:
        raise RuntimeError('less than 40 GiB free; heavy work stopped')
    prepared = args.primary_root / ('private/w45-fb/w46-external-callee-contracts/attempt-1'
                          if args.route == 'system' else 'private/w47-tail/attempt-1/prep')
    source = prepared / 'fluent-bit-clang-source'
    database = prepared / 'fluent-bit-clang-production/compile_commands.json'
    profiles = snapshot / 'contract/profiles/fluent-bit'
    selection = profiles / ('system-selection.json' if args.route == 'system'
                            else 'tail-pause-selection.json')
    clang = '/usr/lib/llvm-21/bin/clang'
    env = dict(os.environ)
    for key in ('QCORE_SOURCE_CONTRACTS', 'QCORE_RETURN_CENSUS', 'QCORE_CHUNKIO_PACKAGE'):
        env.pop(key, None)
    env.update(QCORE_PROFILES=str(snapshot / 'contract/profiles'), QCORE_CLANG=clang,
               QCORE_CROSS_UNIT_LINK_MAP=str(link_map),
               QCORE_NULL_PROPERTY=str(out / 'null.jsonl'),
               QCORE_OUTER_PROJECTION=str(out / 'outer.json'))
    command = [str(binary), 'current', '--checkout', str(source), '--profile-directory', str(profiles),
               '--compile-database', str(database), '--selection', str(selection), '--jobs', '3', '--memory-budget-mib', '7168',
               '--report', str(out / 'resource-diagnostics.json')]
    def identities():
        return {'binary': sha(binary), 'manifest': sha(snapshot / 'manifest.json'),
                'inputs': inputs(), 'source': source_identity(source),
                'database': sha(database), 'selection': sha(selection),
                'link_map': sha(link_map), 'driver': sha(__file__),
                'namespace_wrapper': None}
    record = {'semantic_source': args.semantic_source, 'route': args.route,
              'working_head_before': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
              'command': command, 'cwd': str(root), 'snapshot': str(snapshot),
              'environment': {key: env.get(key, 'unset') for key in
                              ('QCORE_PROFILES', 'QCORE_CLANG', 'QCORE_CROSS_UNIT_LINK_MAP',
                               'QCORE_NULL_PROPERTY', 'QCORE_OUTER_PROJECTION', 'QCORE_SOURCE_CONTRACTS',
                               'QCORE_RETURN_CENSUS', 'QCORE_CHUNKIO_PACKAGE')},
              'clang': subprocess.check_output([clang, '--version'], text=True).splitlines()[0],
              'started': datetime.datetime.now(datetime.timezone.utc).isoformat(),
              'before': identities()}
    (out / 'invocation.json').write_text(json.dumps(record, indent=2) + '\n')
    with (out / 'console.log').open('wb') as stdout, (out / 'stderr.log').open('wb') as stderr:
        code = subprocess.run(command, env=env, stdout=stdout, stderr=stderr).returncode
    record.update(audit_exit=code, after=identities(), ended=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  working_head_after=subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                  free_bytes_after=shutil.disk_usage(root).free)
    record['stable'] = record['before'] == record['after']
    if not record['stable'] or record['free_bytes_after'] < 40 * 1024**3:
        code = 101
    record['exit'] = code
    (out / 'invocation.json').write_text(json.dumps(record, indent=2) + '\n')
    (out / 'exit').write_text(str(code) + '\n')
    return code


if __name__ == '__main__':
    raise SystemExit(main())
