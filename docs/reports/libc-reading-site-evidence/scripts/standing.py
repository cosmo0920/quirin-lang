#!/usr/bin/env python3
"""Standing gate before measurement; authoritative Python follows committed final matrix."""
import hashlib,json,os,shutil,subprocess,sys,time
from pathlib import Path
repo=Path(__file__).resolve().parents[4]
out=repo/'private/libc-reading-site-evidence'/sys.argv[1];out.mkdir(parents=True,exist_ok=False)
source=subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip()
env=dict(os.environ,CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0')
steps=[('submodules',['git','submodule','update','--init','--recursive']),('clang',['clang','--version']),('rust',['python3','scripts/run_test_binaries.py','--jobs','16']),('fmt',['cargo','fmt','--all','--','--check']),('clippy',['cargo','clippy','--all-targets','--','-D','warnings']),('reevaluation-probe',['cargo','run','-p','qcore-compiler','--example','reevaluation_probe']),('reevaluation-followup',['cargo','run','-p','qcore-compiler','--example','reevaluation_followup']),('lean',['python3','scripts/check_lean_audits.py']),('release',['cargo','build','--release','-p','qcore-audit','--bin','quirin-audit'])]
records=[]
for name,command in steps:
    start=time.time();print(name,flush=True)
    with (out/(name+'.log')).open('wb') as log: code=subprocess.run(command,cwd=repo,env=env,stdout=log,stderr=subprocess.STDOUT).returncode
    (out/(name+'.exit')).write_text(str(code)+'\n')
    records.append(dict(step=name,command=command,exit=code,seconds=time.time()-start,source=source,log=str(out/(name+'.log'))))
    (out/'commands.json').write_text(json.dumps(dict(source=source,resource_environment={k:env[k] for k in ['CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_INCREMENTAL']},steps=records,python='deferred until final regenerated matrix committed',providers='N/A: no provider-owned/public schema/qv edits'),indent=2)+'\n')
    print(name,code,flush=True)
    if name=='release' and code==0:
        binary=out/'quirin-audit';shutil.copyfile(repo/'target/release/quirin-audit',binary);binary.chmod(0o755)
        receipt=dict(source=source,binary=str(binary),sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),command=command,feature_selection='default; no feature flags',rustc=subprocess.check_output(['rustc','-vV'],text=True),head_after=subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip())
        (out/'binary.json').write_text(json.dumps(receipt,indent=2)+'\n')
        if receipt['head_after']!=source: raise RuntimeError('source moved during standing build')
raise SystemExit(1 if any(r['exit'] for r in records) else 0)
