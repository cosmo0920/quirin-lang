#!/usr/bin/env python3
"""Measure the joint premise-resolution cycle frozen source/binary on a fresh private route with both projections."""
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


def tree_identity(root):
    return {str(p.relative_to(root)):sha(p) for p in sorted(root.rglob("*")) if p.is_file() and "target" not in p.relative_to(root).parts}


def source_identity(source):
    head = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
    delta = subprocess.check_output(['git', '-C', str(source), 'diff', '--no-ext-diff', '--binary', 'HEAD'])
    return dict(head=head, tracked_delta_sha256=hashlib.sha256(delta).hexdigest())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('route', choices=['corpus', 'system', 'slice'])
    parser.add_argument('out', type=Path)
    parser.add_argument('--link-map', type=Path, required=True)
    parser.add_argument('--inputs-snapshot', type=Path, required=True)
    parser.add_argument('--binary-source', required=True)
    parser.add_argument('--jobs', type=int, default=2)
    args = parser.parse_args()
    assert args.jobs > 0
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
        source = PRIMARY / 'build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source'
    else:
        prepared = PRIMARY / ('private/w45-fb/w46-external-callee-contracts/attempt-1' if route == 'system'
                              else 'private/w47-tail/attempt-1/prep')
        source = prepared / 'fluent-bit-clang-source'
        database = prepared / 'fluent-bit-clang-production/compile_commands.json'
        selection = snapshot / 'contract/profiles/fluent-bit' / ('system-selection.json' if route == 'system'
                                                          else 'tail-pause-selection.json')
        command = [str(binary), 'current', '--checkout', str(source), '--profile-directory',
                   str(snapshot / 'contract/profiles/fluent-bit'), '--compile-database', str(database),
                   '--selection', str(selection), '--jobs', str(args.jobs), '--report', str(out / 'resource-diagnostics.json')]
    record = {'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip(),
              'analyzer_source': args.binary_source, 'inputs_snapshot': str(snapshot), 'inputs_before':tree_identity(snapshot),
              'report_driver_sha256': sha(Path(__file__)),
              'runner_sha256': sha(snapshot / 'scripts/run_pinned_audit.py'),
              'profile_sha256': sha(snapshot / 'contract/profiles/fluent-bit/profile.json'),
              'binary': str(binary), 'binary_sha256': sha(binary), 'command': command, 'cwd': str(REPO), 'jobs': args.jobs,
              'source_root':str(source), 'source_head': subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip(),
              'source_identity': source_identity(source),
              'link_map': str(link_map), 'link_map_sha256': sha(link_map),
              'environment': {k: env.get(k, 'unset') for k in ('QCORE_PROFILES', 'QCORE_CLANG', 'QCORE_CROSS_UNIT_LINK_MAP',
                              'QCORE_NULL_PROPERTY', 'QCORE_OUTER_PROJECTION', 'QCORE_SOURCE_CONTRACTS', 'QCORE_RETURN_CENSUS', 'QCORE_CHUNKIO_PACKAGE', 'QCORE_AUDIT_JOBS')},
              'started': datetime.datetime.now(datetime.timezone.utc).isoformat()}
    if route == 'corpus':
        record['driver_environment_overrides']={'QCORE_CLANG':'/usr/bin/clang','QCORE_CHUNKIO_PACKAGE':str(PRIMARY/'build/cycle19/provider-packages-wave20/qcore.chunkio')}
        record['external_inputs_before']={str(p):sha(p) for p in [source/'build/compile_commands.json',PRIMARY/'build/cycle19/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json']}
        record['package_inputs_before']=tree_identity(PRIMARY/'build/cycle19/provider-packages-wave20/qcore.chunkio')
    if route != 'corpus':
        record.update(compile_database=str(database), compile_database_sha256=sha(database),
                      selection=str(selection), selection_sha256=sha(selection))
    (out / 'invocation.json').write_text(json.dumps(record, indent=2) + '\n')
    with (out / 'console.log').open('w') as stdout, (out / 'stderr.log').open('w') as stderr:
        result = subprocess.run(command, cwd=REPO, env=env, stdout=stdout, stderr=stderr)
    record.update(exit=result.returncode, ended=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  binary_sha256_after=sha(binary),
                  head_after=subprocess.check_output(['git','rev-parse','HEAD'],cwd=REPO,text=True).strip())
    record['inputs_after']=tree_identity(snapshot)
    record['report_driver_sha256_after'] = sha(Path(__file__))
    record['source_identity_after'] = source_identity(source)
    record['runner_sha256_after'] = sha(snapshot / 'scripts/run_pinned_audit.py')
    record['profile_sha256_after'] = sha(snapshot / 'contract/profiles/fluent-bit/profile.json')
    record['link_map_sha256_after'] = sha(link_map)
    if route != 'corpus':
        record['compile_database_sha256_after'] = sha(database)
        record['selection_sha256_after'] = sha(selection)
    if route == 'corpus':
        record['external_inputs_after']={p:sha(Path(p)) for p in record['external_inputs_before']}
        record['package_inputs_after']=tree_identity(PRIMARY/'build/cycle19/provider-packages-wave20/qcore.chunkio')
        record['retained_actual_driver_receipt']=str(out/'corpus/run.json')
        record['actual_driver_receipt']=json.loads((out/'corpus/run.json').read_bytes())
        assert record['actual_driver_receipt']['exit']==record['exit'] and record['actual_driver_receipt']['binary_sha256']==record['binary_sha256']
        assert record['external_inputs_before']==record['external_inputs_after'] and record['package_inputs_before']==record['package_inputs_after']
    stable = record['inputs_before']==record['inputs_after'] and all(record.get(k + '_after') == record[k] for k in ['source_identity', 'runner_sha256', 'profile_sha256', 'link_map_sha256', 'report_driver_sha256'] + ([] if route == 'corpus' else ['compile_database_sha256', 'selection_sha256']))
    if not stable or record['binary_sha256_after'] != record['binary_sha256'] or record['head_after'] != record['head']:
        record['identity_error'] = 'source or binary changed during route'
        record['audit_exit'] = result.returncode
        result.returncode = 101
        record['exit'] = 101
    (out / 'invocation.json').write_text(json.dumps(record, indent=2) + '\n')
    (out / f'{route}.exit').write_text(str(result.returncode) + '\n')
    return result.returncode


if __name__ == '__main__':
    raise SystemExit(main())
