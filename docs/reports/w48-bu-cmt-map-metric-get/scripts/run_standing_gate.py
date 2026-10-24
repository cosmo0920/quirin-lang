"""Run only as a descendant of this card's acquired development-lock shell."""
from pathlib import Path
import datetime
import json
import os
import shutil
import subprocess
import sys
import time

root = Path.cwd()
evidence = root / 'private/w48-cmt-map'
slot = json.loads((evidence / 'development-lock.json').read_text())
assert slot['state'] == 'development_slot_acquired'
ancestor = os.getppid()
while ancestor > 1 and ancestor != slot['pid']:
    ancestor = int(Path(f'/proc/{ancestor}/stat').read_text().rsplit(')', 1)[1].split()[1])
assert ancestor == slot['pid'], 'gate must be a child of the held validation slot'
assert shutil.disk_usage(root).free >= 40 * 1024**3
assert not subprocess.check_output(['git', 'status', '--porcelain'], text=True).strip()
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ')
out = evidence / 'gate' / stamp
out.mkdir(parents=True, exist_ok=False)
env = dict(os.environ, CARGO_BUILD_JOBS='6', QCORE_CLANG='/usr/lib/llvm-21/bin/clang')
commands = [
    ('submodules', ['git', 'submodule', 'update', '--init', '--recursive']),
    ('clang-version', [env['QCORE_CLANG'], '--version']),
    ('rust-tests', ['python3', 'scripts/run_test_binaries.py', '--jobs', '6', '--log-dir', str(out / 'rust-binaries')]),
    ('fmt', ['cargo', 'fmt', '--all', '--', '--check']),
    ('clippy', ['cargo', 'clippy', '--all-targets', '--', '-D', 'warnings']),
    ('reevaluation-probe', ['cargo', 'run', '-p', 'qcore-compiler', '--example', 'reevaluation_probe']),
    ('reevaluation-followup', ['cargo', 'run', '-p', 'qcore-compiler', '--example', 'reevaluation_followup']),
    ('lean-audits', ['python3', 'scripts/check_lean_audits.py', '--jobs', '6']),
    ('python-tests', ['python3', '-m', 'unittest', 'discover', '-s', 'scripts/tests']),
]
record = {'head': head, 'started': stamp, 'cwd': str(root), 'environment_overrides': {k: env[k] for k in ['CARGO_BUILD_JOBS', 'QCORE_CLANG']}, 'steps': []}
for name, command in commands:
    assert shutil.disk_usage(root).free >= 40 * 1024**3
    assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip() == head
    assert not subprocess.check_output(['git', 'status', '--porcelain'], text=True).strip()
    started = time.monotonic()
    print(f'START {name}: {command}', flush=True)
    log = out / (name + ('.txt' if name == 'clang-version' else '.log'))
    with log.open('wb') as stream:
        code = subprocess.run(command, env=env, stdout=stream, stderr=stream).returncode
    (out / (name + '.exit')).write_text(str(code) + '\n')
    record['steps'].append({'name': name, 'command': command, 'exit': code, 'seconds': time.monotonic() - started, 'log': str(log)})
    (out / 'gate-run.json').write_text(json.dumps(record, indent=2) + '\n')
    print(f'END {name}: exit {code}', flush=True)
record['head_after'] = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
record['clean_after'] = not subprocess.check_output(['git', 'status', '--porcelain'], text=True).strip()
record['exit'] = 0 if record['clean_after'] and record['head_after'] == head and all(s['exit'] == 0 for s in record['steps']) else 1
(out / 'gate-run.json').write_text(json.dumps(record, indent=2) + '\n')
(evidence / 'latest-gate.json').write_text(json.dumps({'path': str(out), 'exit': record['exit'], 'head': head}, indent=2) + '\n')
raise SystemExit(record['exit'])
