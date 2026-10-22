#!/usr/bin/env python3
"""Build exact archived BASE and freeze its analyzer/runtime inputs under lock."""
import datetime,fcntl,hashlib,json,os,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
OUT=ROOT/'private/guarded-contract-adoption/native-base-build-1'
record=json.loads((OUT/'source-receipt.json').read_bytes());source=record['source'];checkout=Path(record['checkout'])
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def stable():
    for name,digest in record['files'].items():assert sha(checkout/name)==digest,name
with open('/tmp/qcore-c-card-validation-20261022.lock','a') as lock:
    print('WAITING FOR BASE BUILD LOCK',flush=True);fcntl.flock(lock,fcntl.LOCK_EX)
    stable();start=datetime.datetime.now(datetime.timezone.utc).isoformat()
    command=['cargo','build','--release','--locked','--offline','-p','qcore-audit','--bin','quirin-audit','--message-format=json','-j2']
    overrides=dict(CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',QCORE_AUDIT_BUILD_COMMIT=source)
    env=dict(os.environ,**overrides)
    with (OUT/'build.stdout.log').open('wb') as stdout,(OUT/'build.stderr.log').open('wb') as stderr:
        result=subprocess.run(command,cwd=checkout,env=env,stdout=stdout,stderr=stderr)
    stable();binary=checkout/'target/release/quirin-audit'
    receipt={'source':source,'build_head':source,'archive_receipt_sha256':sha(OUT/'source-receipt.json'),'archive_source_files_unchanged':True,'archive_files':len(record['files']),'build':{'command':command,'cwd':str(checkout),'environment_overrides':overrides,'started':start,'ended':datetime.datetime.now(datetime.timezone.utc).isoformat(),'exit':result.returncode,'stdout':str(OUT/'build.stdout.log'),'stderr':str(OUT/'build.stderr.log')},'binary':str(binary),'sha256':sha(binary) if binary.exists() else None}
    (OUT/'binary-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');assert result.returncode==0
    freeze=['python3',str(ROOT/'docs/reports/premise-symbolic-integration/scripts/freeze_inputs.py'),str(OUT/'freeze'),'--source',source,'--binary',str(binary),'--binary-sha256',receipt['sha256'],'--binary-receipt',str(OUT/'binary-receipt.json')]
    with (OUT/'freeze.stdout.log').open('wb') as stdout,(OUT/'freeze.stderr.log').open('wb') as stderr:r=subprocess.run(freeze,cwd=ROOT,stdout=stdout,stderr=stderr)
    assert r.returncode==0
    (OUT/'completion.json').write_text(json.dumps({'exit':0,'build_receipt_sha256':sha(OUT/'binary-receipt.json'),'freeze_receipt_sha256':sha(OUT/'freeze/receipt.json'),'completed':datetime.datetime.now(datetime.timezone.utc).isoformat()},indent=2)+'\n')
    print(json.dumps(receipt),flush=True)
