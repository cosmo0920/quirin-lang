#!/usr/bin/env python3
import hashlib,json,os,subprocess,time,sys
from pathlib import Path
repo=Path(__file__).resolve().parents[4]
out=repo/'private/libc-reading-site-evidence'/sys.argv[1]
out.mkdir(parents=True,exist_ok=False)
env=dict(os.environ,CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0')
steps=[('format',['cargo','fmt','--all'],repo),('reader-tests',['cargo','test','-p','qcore-c-gateway','--test','libc_reading_site_evidence','--','--nocapture'],repo),('recorder-tests',['cargo','test','-p','qcore-c-gateway','--lib','reading_site_evidence','--','--nocapture'],repo),('probe-tests',['cargo','test','-p','qcore-c-gateway','--lib','probe_state::tests','--','--nocapture'],repo),('lean',['lake','build','QCore.LibcReadingSiteEvidence'],repo/'proof')]
receipts=[]
failed=False
for name,command,cwd in steps:
    start=time.time()
    with (out/(name+'.log')).open('wb') as log:
        code=subprocess.run(command,cwd=cwd,env=env,stdout=log,stderr=subprocess.STDOUT).returncode
    receipts.append(dict(step=name,command=command,cwd=str(cwd),exit=code,seconds=time.time()-start))
    (out/'commands.json').write_text(json.dumps(dict(resource_environment={k:env[k] for k in ['CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_INCREMENTAL']},steps=receipts),indent=2)+'\n')
    print(name,code,flush=True)
    if name=='format' and code==0:
        p=repo/'docs/reports/libc-reading-site-evidence/bindings.json';m=json.loads(p.read_text())
        for reference in m['artifacts'].values(): reference['sha256']=hashlib.sha256((repo/reference['path']).read_bytes()).hexdigest()
        p.write_text(json.dumps(m,indent=2)+'\n')
    if code: failed=True
raise SystemExit(1 if failed else 0)
