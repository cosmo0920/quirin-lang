#!/usr/bin/env python3
"""Finite logger model controls; raw sources/logs remain in a fresh private run."""
import argparse
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess


def sha(b):
    return hashlib.sha256(b).hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--source-root', type=Path, required=True)
    ap.add_argument('--out', type=Path, required=True)
    args = ap.parse_args()
    root = Path(__file__).resolve().parents[3]
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    source = args.source_root.resolve()
    document = root / 'docs/reports/logger-effect-contracts/model.json'
    data = document.read_bytes()
    model = json.loads(data)
    expected = '1c1ffedb049c363f1003c3eaa1e3110c08828375ab85e12e647578a3f423ed18'
    assert sha(data) == expected
    revision = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
    assert revision == model['revision']
    source_before = {}
    for name, pin in model['sources'].items():
        actual = (source / name).read_bytes()
        committed = subprocess.check_output(['git', '-C', str(source), 'show', f'{revision}:{name}'])
        assert actual == committed and sha(actual) == pin['sha256']
        source_before[name] = sha(actual)
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    env = dict(os.environ, CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0')
    records = []

    def run(label, argv, cwd=root, expected_exit=0):
        record = {'label': label, 'argv': argv, 'cwd': str(cwd), 'source_head': head,
                  'started_utc': datetime.datetime.now(datetime.timezone.utc).isoformat()}
        with (out / f'{label}.stdout').open('wb') as so, (out / f'{label}.stderr').open('wb') as se:
            process = subprocess.Popen(argv, cwd=cwd, env=env, stdout=so, stderr=se)
            record['pid'] = process.pid
            record['exit'] = process.wait()
        record['finished_utc'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
        for channel in ['stdout', 'stderr']:
            file = out / f'{label}.{channel}'
            record[channel] = {'path': str(file), 'sha256': sha(file.read_bytes())}
        records.append(record)
        (out / 'commands.json').write_text(json.dumps(records, indent=2) + '\n')
        if expected_exit == 'nonzero':
            assert record['exit'] != 0, record
        else:
            assert record['exit'] == expected_exit, record

    with open('/tmp/qcore-c-card-validation-20261022.lock', 'a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        run('fmt', ['cargo', 'fmt', '--all', '--', '--check'])
        run('rust', ['cargo', 'test', '-p', 'qcore-contract', '--test', 'logger_effects', '--no-fail-fast'])
        run('clippy', ['cargo', 'clippy', '-p', 'qcore-contract', '--all-targets', '--', '-D', 'warnings'])
        run('lean-version', ['lake', 'env', 'lean', '--version'], root / 'proof')
        assert '4.35.0' in (out / 'lean-version.stdout').read_text()
        run('lean-model', ['lake', 'build', 'QCore.LoggerEffects'], root / 'proof')
        run('lean-audit', ['lake', 'env', 'lean', 'AuditLoggerEffects.lean'], root / 'proof')
        run('source-replay', ['cargo', 'run', '-p', 'qcore-contract', '--example', 'logger_effect_plan', '--',
            str(document), expected, str(source)])
        run('wrong-model-digest', [str(root / 'target/debug/examples/logger_effect_plan'), str(document), '0' * 64, str(source)], expected_exit='nonzero')
        altered = out / 'altered-source'
        for name in model['sources']:
            dest = altered / name
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes((source / name).read_bytes())
        changed = altered / next(iter(model['sources']))
        changed.write_bytes(changed.read_bytes() + b'\n/* changed source */\n')
        run('wrong-source', [str(root / 'target/debug/examples/logger_effect_plan'), str(document), expected, str(altered)], expected_exit='nonzero')
    source_after = {name: sha((source / name).read_bytes()) for name in model['sources']}
    assert source_after == source_before
    assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip() == head
    receipt = {'source_head': head, 'model_sha256': expected, 'source_root': str(source),
        'reviewed_revision': revision, 'source_before': source_before, 'source_after': source_after,
        'commands': records, 'source_binding': 'Open; exact raw input identity only',
        'production_consumer': False, 'standing_gate': 'pending integrator combined gate'}
    (out / 'completion.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps({'completed': str(out / 'completion.json'), 'commands': len(records)}))


if __name__ == '__main__':
    main()
