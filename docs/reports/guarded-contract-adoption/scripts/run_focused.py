#!/usr/bin/env python3
"""Run card-focused commands serially with exact exits and private receipts."""
import argparse
import datetime as dt
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[4]
parser = argparse.ArgumentParser()
parser.add_argument('run')
parser.add_argument('--pins', action='store_true')
args = parser.parse_args()
out = ROOT / 'private/guarded-contract-adoption' / args.run
out.mkdir(parents=True, exist_ok=False)
env = os.environ.copy()
env.update(CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0', CARGO_BUILD_JOBS='2')
if args.pins:
    commands = [
        (ROOT, ['target/debug/quirin-audit','pin','--source-root',source,'--crate','qcore-provider-librdkafka','--function','cJSON_DetachItemViaPointer'])
        for source in [
            '/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source',
            '/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source',
        ]
    ] + [
        (ROOT,['python3','providers/contracts.py','sync']),
        (ROOT,['python3','providers/contracts.py','check']),
    ]
else:
    commands = [
        (ROOT,['cargo','test','-p','qcore-contract','--no-fail-fast','--test','guarded_contract_adoption','--test','guarded_decomposition','--test','premise_metadata_bridge','--test','contract_views','--test','premise_flow','-j','2']),
        (ROOT,['cargo','fmt','--all','--','--check']),
        (ROOT,['cargo','clippy','-p','qcore-contract','--all-targets','--','-D','warnings']),
        (ROOT/'proof',['lake','build','QCore.GuardedSourceAdoption']),
        (ROOT/'proof',['lake','env','lean','AuditGuardedSourceAdoption.lean']),
    ]
rows=[]
with open('/tmp/qcore-c-card-validation-20261022.lock','a') as lock:
    fcntl.flock(lock,fcntl.LOCK_EX)
    for i,(cwd,command) in enumerate(commands):
        start=dt.datetime.now(dt.timezone.utc).isoformat()
        log=out/f'{i:02}.log'
        with log.open('wb') as stream:
            result=subprocess.run(command,cwd=cwd,env=env,stdout=stream,stderr=subprocess.STDOUT)
        row=dict(command=command,cwd=str(cwd),start=start,end=dt.datetime.now(dt.timezone.utc).isoformat(),exit=result.returncode,log=str(log),log_sha256=hashlib.sha256(log.read_bytes()).hexdigest(),head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip())
        rows.append(row)
        (out/'commands.json').write_text(json.dumps(rows,indent=2)+'\n')
        print(json.dumps(row),flush=True)
        if result.returncode:
            raise SystemExit(result.returncode)
