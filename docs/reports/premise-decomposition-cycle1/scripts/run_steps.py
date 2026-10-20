#!/usr/bin/env python3
"""Run declared steps with exact exit/log receipts; never pipe a gate."""
import argparse, datetime, json, os
from pathlib import Path
import subprocess
ROOT=Path(__file__).resolve().parents[4]
p=argparse.ArgumentParser();p.add_argument('stage',choices=['prep','standing','provider','python']);p.add_argument('out',type=Path);a=p.parse_args()
out=a.out.resolve();out.mkdir(parents=True,exist_ok=False)
source=Path('/home/cosmo/Documents/GitHub/qcore-c')
steps={
'prep':[
 ['target/release/quirin-audit','pin','--source-root',str(source/'build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source'),'--crate','qcore-provider-librdkafka','--function','cJSON_DetachItemViaPointer'],
 ['target/release/quirin-audit','pin','--source-root',str(source/'private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source'),'--crate','qcore-provider-librdkafka','--function','cJSON_DetachItemViaPointer'],
 ['python3','providers/contracts.py','sync'],['python3','providers/contracts.py','check']],
'standing':[['git','submodule','update','--init','--recursive'],['clang','--version'],['python3','scripts/run_test_binaries.py','--jobs','4'],['cargo','fmt','--all','--','--check'],['cargo','clippy','--all-targets','--','-D','warnings'],['cargo','run','-p','qcore-compiler','--example','reevaluation_probe'],['cargo','run','-p','qcore-compiler','--example','reevaluation_followup'],['python3','scripts/check_lean_audits.py','--jobs','4']],
'provider':[['cargo','test','--manifest-path','providers/Cargo.toml','--workspace','--locked','--offline','--no-fail-fast'],['cargo','fmt','--manifest-path','providers/Cargo.toml','--all','--','--check'],['cargo','clippy','--manifest-path','providers/Cargo.toml','--workspace','--all-targets','--locked','--offline','--','-D','warnings'],['python3','-m','unittest','discover','-s','providers/tests'],['python3','providers/contracts.py','check'],['python3','providers/package.py','build','--output',str(out/'packages')]]+[['python3','providers/package.py','verify',str(out/'packages'/name)] for name in ['qcore.chunkio','qcore.libevent','qcore.librdkafka','qcore.cmetrics','qcore.ctraces']],
'python':[['python3','-m','unittest','discover','-s','scripts/tests']]}
# The rejected companion's configured tips remain archived separately. Reuse
# the immutable original-input baselines, but take fresh final-input tips before
# this same exclusive validation sequence releases its advisory lock.
if a.stage == 'provider':
    final_head = subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    steps['provider'] += [[os.sys.executable,str(Path(__file__).with_name('measure.py')),route,
        str(ROOT/'target/release/quirin-audit'),final_head,str(ROOT),
        str(out/'final-configured'/route/'tip')] for route in ['system','slice']]
receipts=[]
for i,command in enumerate(steps[a.stage]):
 log=out/f'{i:02}.log';start=datetime.datetime.now(datetime.timezone.utc).isoformat()
 with log.open('wb') as f:
  result=subprocess.run(command,cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
 receipt=dict(command=command,cwd=str(ROOT),exit=result.returncode,log=str(log),start=start,end=datetime.datetime.now(datetime.timezone.utc).isoformat())
 receipts.append(receipt);(out/'commands.json').write_text(json.dumps(receipts,indent=2)+'\n');(out/f'{i:02}.exit').write_text(str(result.returncode)+'\n')
 print(json.dumps(receipt),flush=True)
 if result.returncode and a.stage=='prep':raise SystemExit(result.returncode)
raise SystemExit(0 if all(r['exit']==0 for r in receipts) else 1)
