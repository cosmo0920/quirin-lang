#!/usr/bin/env python3
"""Queue behind the frozen final gate and fail closed before native acceptance."""
import datetime,fcntl,hashlib,json,subprocess,sys,traceback
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];SCRIPTS=ROOT/'docs/reports/guarded-contract-adoption/scripts'
EXPECTED='552773f0c64e2e6a3c7d4b752c61a7d26130975a'
GATE=Path('/home/cosmo/.codex/worktrees/logger-effect-contracts/qcore-c/private/contract-cycle4-integration/joint-gate-2')
OUT=ROOT/'private/guarded-contract-adoption/native-controller-1'
OUT.mkdir(parents=True,exist_ok=False)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda p:json.loads(p.read_bytes())
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip();records=[]
def guard():
 assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()==head
 assert not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True)
def run(name,args):
 guard();print('START',name,flush=True);start=datetime.datetime.now(datetime.timezone.utc).isoformat()
 with (OUT/(name+'.stdout.log')).open('wb') as stdout,(OUT/(name+'.stderr.log')).open('wb') as stderr:r=subprocess.run([sys.executable,*map(str,args)],cwd=ROOT,stdout=stdout,stderr=stderr)
 records.append({'name':name,'command':[sys.executable,*map(str,args)],'cwd':str(ROOT),'started':start,'ended':datetime.datetime.now(datetime.timezone.utc).isoformat(),'exit':r.returncode});(OUT/'commands.json').write_text(json.dumps(records,indent=2)+'\n');guard();print('END',name,r.returncode,flush=True);assert r.returncode==0,name
try:
 with open('/tmp/qcore-c-card-validation-20261022.lock','a') as lock:
  print('WAITING BEHIND FINAL GATE',flush=True);fcntl.flock(lock,fcntl.LOCK_EX);guard()
  completion=read(GATE/'completion.json');commands=read(GATE/'commands.json')
  assert completion['wrapper_exit']==0 and completion['source_stable'] and completion['source']==EXPECTED and completion['head_after']==EXPECTED and completion['clean_after']
  assert completion['commands_sha256']==sha(GATE/'commands.json') and len(commands)==22 and all(v['exit']==0 for v in commands)
  base=ROOT/'private/guarded-contract-adoption/native-base-build-1';assert read(base/'completion.json')['exit']==0
  assert read(base/'binary-receipt.json')['source']=='31c5ed500d9d4a1448a623b839e1644adb19efd1'
  (OUT/'gate-acceptance.json').write_text(json.dumps({'source':EXPECTED,'completion_sha256':sha(GATE/'completion.json'),'commands_sha256':sha(GATE/'commands.json'),'gate':str(GATE),'all_22_commands_zero':True,'reporting_head':head,'checked':datetime.datetime.now(datetime.timezone.utc).isoformat()},indent=2)+'\n')
 prep=ROOT/'private/guarded-contract-adoption/native-tip-preparation-1';base_run=ROOT/'private/guarded-contract-adoption/native-base-1';tip_run=ROOT/'private/guarded-contract-adoption/native-tip-1'
 run('prepare-tip',[SCRIPTS/'prepare_tip.py','--gate',GATE,'--run',prep.name])
 run('base',[SCRIPTS/'run_side.py','base','--freeze',base/'freeze','--build-receipt',base/'binary-receipt.json','--run',base_run.name])
 run('tip',[SCRIPTS/'run_side.py','tip','--freeze',prep/'freeze','--build-receipt',prep/'binary-receipt.json','--run',tip_run.name])
 run('compare',[SCRIPTS/'compare_all.py','--base',base_run,'--tip',tip_run,'--base-build',base/'binary-receipt.json','--tip-preparation',prep,'--run','native-comparison-1'])
 (OUT/'completion.json').write_text(json.dumps({'exit':0,'source':EXPECTED,'reporting_head':head,'commands_sha256':sha(OUT/'commands.json'),'completed':datetime.datetime.now(datetime.timezone.utc).isoformat()},indent=2)+'\n')
except BaseException:
 (OUT/'failure.txt').write_text(traceback.format_exc());print('CAMPAIGN STOPPED WITH RETAINED FAILURE',flush=True);raise
