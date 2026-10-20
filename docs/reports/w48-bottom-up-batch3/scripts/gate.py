#!/usr/bin/env python3
"""Run standing submodule/Rust commands after audits finish. Other source commands and Lean have separate receipts; Python follows matrix regeneration."""
from pathlib import Path
import datetime
import json
import os
import subprocess
import sys

REPO = Path(__file__).resolve().parents[4]
out = (REPO / sys.argv[1]).resolve()
out.mkdir(parents=True, exist_ok=False)
env = dict(os.environ, CARGO_BUILD_JOBS='2', CARGO_PROFILE_DEV_DEBUG='0',
           CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0', QCORE_CLANG='/usr/lib/llvm-21/bin/clang')
commands = [
    ('submodules', ['git', 'submodule', 'update', '--init', '--recursive']),
    ('rust', ['python3', 'scripts/run_test_binaries.py', '--jobs', '4', '--log-dir', str(out/'rust-units')]),
]
records = []
for name, command in commands:
    record = dict(name=name, command=command, started=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  source=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip(), log=str(out/(name+'.log')))
    with (out/(name+'.log')).open('w') as stream:
        run = subprocess.run(command, cwd=REPO, env=env, stdout=stream, stderr=stream)
    record.update(exit=run.returncode, ended=datetime.datetime.now(datetime.timezone.utc).isoformat())
    (out/(name+'.exit')).write_text(str(run.returncode)+'\n')
    records.append(record)
    (out/'commands.json').write_text(json.dumps(records, indent=2)+'\n')
    print(f'{name}: {run.returncode}', flush=True)
raise SystemExit(0 if all(r['exit']==0 for r in records) else 1)
