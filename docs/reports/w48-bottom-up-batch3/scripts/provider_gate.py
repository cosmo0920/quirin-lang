#!/usr/bin/env python3
"""Validate the provider dependency workspace and all five packages."""
from pathlib import Path
import json
import os
import subprocess
import sys

REPO = Path(__file__).resolve().parents[4]
out = (REPO / sys.argv[1]).resolve()
out.mkdir(parents=True, exist_ok=False)
env = dict(os.environ, CARGO_BUILD_JOBS='2', CARGO_PROFILE_DEV_DEBUG='0',
           CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0')
manifest = ['--manifest-path', 'providers/Cargo.toml']
steps = [
    ('check', ['cargo','check', *manifest,'--workspace','--locked','--offline']),
    ('rust', ['cargo','test', *manifest,'--workspace','--locked','--offline','--no-fail-fast']),
    ('fmt', ['cargo','fmt', *manifest,'--all','--','--check']),
    ('clippy', ['cargo','clippy', *manifest,'--workspace','--all-targets','--locked','--offline','--','-D','warnings']),
    ('sync', ['python3','providers/contracts.py','sync']),
    ('contracts', ['python3','providers/contracts.py','check']),
    ('python', ['python3','-m','unittest','discover','-s','providers/tests']),
    ('packages', ['python3','providers/package.py','build','--output',str(out/'packages'), '--target-dir',str(REPO/'private/w48-bottom-up-batch3/provider-release-target')]),
]
for package in ['qcore.libevent','qcore.chunkio','qcore.cmetrics','qcore.ctraces','qcore.librdkafka']:
    steps.append(('verify-'+package, ['python3','providers/package.py','verify',str(out/'packages'/package)]))
records = []
for name, command in steps:
    log = out/(name+'.log')
    with log.open('w') as stream:
        run = subprocess.run(command,cwd=REPO,env=env,stdout=stream,stderr=stream)
    (out/(name+'.exit')).write_text(str(run.returncode)+'\n')
    records.append(dict(name=name, command=command, exit=run.returncode, private_log=str(log)))
    (out/'commands.json').write_text(json.dumps(records,indent=2)+'\n')
    print(f'{name}: {run.returncode}',flush=True)
raise SystemExit(0 if all(x['exit']==0 for x in records) else 1)
