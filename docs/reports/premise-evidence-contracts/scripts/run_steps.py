#!/usr/bin/env python3
"""Exact no-pipe commands/exits/logs for this bounded card, including failures."""
import argparse,datetime,json,os,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
p=argparse.ArgumentParser();p.add_argument('stage',choices=['focused','pins','standing','provider','python']);p.add_argument('out',type=Path);a=p.parse_args();out=a.out.resolve();out.mkdir(parents=True,exist_ok=False)
primary=Path('/home/cosmo/Documents/GitHub/qcore-c')
steps={
'focused':[(ROOT,['cargo','test','-p','qcore-contract','--test','hypothesis','--test','hypothesis_bindings','--no-fail-fast']),(ROOT,['cargo','test','-p','qcore-audit','--test','contract_dependencies_report','--no-fail-fast']),(ROOT/'proof',['lake','build','QCore.PremiseRefinement']),(ROOT/'proof',['lake','env','lean','AuditPremiseRefinement.lean']),(ROOT,['cargo','build','--release','-p','qcore-audit','--bin','quirin-audit'])],
'pins':[(ROOT,['target/release/quirin-audit','pin','--source-root',str(primary/path),'--crate','qcore-provider-librdkafka','--function','cJSON_DetachItemViaPointer']) for path in ['build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source','private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source']]+[(ROOT,['python3','providers/contracts.py','sync']),(ROOT,['python3','providers/contracts.py','check'])],
'standing':[(ROOT,c) for c in [['git','submodule','update','--init','--recursive'],['clang','--version'],['python3','scripts/run_test_binaries.py','--jobs','4'],['cargo','fmt','--all','--','--check'],['cargo','clippy','--all-targets','--','-D','warnings'],['cargo','run','-p','qcore-compiler','--example','reevaluation_probe'],['cargo','run','-p','qcore-compiler','--example','reevaluation_followup'],['python3','scripts/check_lean_audits.py','--jobs','4']]],
'provider':[(ROOT,c) for c in [['cargo','test','--manifest-path','providers/Cargo.toml','--workspace','--locked','--offline','--no-fail-fast'],['cargo','fmt','--manifest-path','providers/Cargo.toml','--all','--','--check'],['cargo','clippy','--manifest-path','providers/Cargo.toml','--workspace','--all-targets','--locked','--offline','--','-D','warnings'],['python3','-m','unittest','discover','-s','providers/tests'],['python3','providers/contracts.py','check'],['python3','providers/package.py','build','--output',str(out/'packages')]]]+[(ROOT,['python3','providers/package.py','verify',str(out/'packages'/name)]) for name in ['qcore.chunkio','qcore.libevent','qcore.librdkafka','qcore.cmetrics','qcore.ctraces']],
'python':[(ROOT,['python3','-m','unittest','discover','-s','scripts/tests'])]}
env=dict(os.environ);env.update(CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0')
receipts=[]
for i,(cwd,command) in enumerate(steps[a.stage]):
 start=datetime.datetime.now(datetime.timezone.utc).isoformat();log=out/f'{i:02}.log'
 with log.open('wb') as stream:code=subprocess.run(command,cwd=cwd,env=env,stdout=stream,stderr=subprocess.STDOUT).returncode
 row=dict(command=command,cwd=str(cwd),environment={k:env[k] for k in ['CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_INCREMENTAL']},start=start,end=datetime.datetime.now(datetime.timezone.utc).isoformat(),exit=code,log=str(log),head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip())
 receipts.append(row);(out/'commands.json').write_text(json.dumps(receipts,indent=2)+'\n');(out/f'{i:02}.exit').write_text(str(code)+'\n');print(json.dumps(row),flush=True)
 if code and a.stage=='pins':raise SystemExit(code)
raise SystemExit(0 if all(row['exit']==0 for row in receipts) else 1)
