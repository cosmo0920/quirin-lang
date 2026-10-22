#!/usr/bin/env python3
"""Freeze final successful gated TIP before exact-artifact helper discovery."""
import argparse,datetime,fcntl,hashlib,json,os,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
EXTERNS=['qcore_audit','qcore_c_gateway','qcore_contract','qcore_schemas','serde_json']
p=argparse.ArgumentParser();p.add_argument('--gate',type=Path,required=True);p.add_argument('--run',required=True);a=p.parse_args()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
gate=a.gate.resolve();completion=json.loads((gate/'completion.json').read_bytes());commands=json.loads((gate/'commands.json').read_bytes());assert completion['wrapper_exit']==0 and completion['source_stable'];assert completion['commands_sha256']==sha(gate/'commands.json');assert len(commands)==22 and all(r['exit']==0 for r in commands)
release=next(r for r in commands if r['name']=='release');workspace=Path(release['cwd']);source=completion['source'];assert release['head_after']==source and release['clean_after'];assert release['environment_overrides']['QCORE_AUDIT_BUILD_COMMIT']==source
out=ROOT/'private/guarded-contract-adoption'/a.run;out.mkdir(parents=True,exist_ok=False);records=[]
def own_guard():
 assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()==own_head
 assert not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True)
def root_code_guard():
 for path in ['Cargo.toml','Cargo.lock','compiler','crates','proof','contract','providers','scripts','cycle19-resource-contracts.json']:
  assert subprocess.check_output(['git','rev-parse',source+':'+path],cwd=workspace)==subprocess.check_output(['git','rev-parse','HEAD:'+path],cwd=workspace)
  assert not subprocess.check_output(['git','diff','--name-only',source,'--',path],cwd=workspace)
def run(name,command,cwd,env):
 own_guard();root_code_guard()
 with (out/(name+'.stdout.log')).open('wb') as stdout,(out/(name+'.stderr.log')).open('wb') as stderr:r=subprocess.run(command,cwd=cwd,env=env,stdout=stdout,stderr=stderr)
 records.append({'name':name,'command':list(map(str,command)),'cwd':str(cwd),'exit':r.returncode,'completed':datetime.datetime.now(datetime.timezone.utc).isoformat()});(out/'commands.json').write_text(json.dumps(records,indent=2)+'\n');own_guard();root_code_guard();assert r.returncode==0,name
with open('/tmp/qcore-c-card-validation-20261022.lock','a') as lock:
 print('WAITING FOR FINAL TIP FREEZE LOCK',flush=True);fcntl.flock(lock,fcntl.LOCK_EX)
 own_head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip();own_guard();root_code_guard()
 binary=workspace/'target/release/quirin-audit';binary_sha=sha(binary)
 receipt={'source':source,'sha256':binary_sha,'binary':str(binary),'build':release,'gate_completion_sha256':sha(gate/'completion.json'),'gate_commands_sha256':sha(gate/'commands.json'),'source_code_equal_at_freeze':True,'reporting_source':own_head}
 (out/'binary-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
 env=dict(os.environ,**release['environment_overrides'])
 run('freeze',[sys.executable,ROOT/'docs/reports/premise-symbolic-integration/scripts/freeze_inputs.py',out/'freeze','--source',source,'--binary',binary,'--binary-sha256',binary_sha,'--binary-receipt',out/'binary-receipt.json'],ROOT,env)
 run('cargo-artifacts',['cargo','build','--release','-p','qcore-audit','--bin','quirin-audit','--message-format=json','-j2'],workspace,env)
 assert sha(binary)==binary_sha,'artifact discovery changed the gated analyzer binary'
 artifacts={}
 for line in (out/'cargo-artifacts.stdout.log').read_bytes().splitlines():
  value=json.loads(line)
  if value.get('reason')!='compiler-artifact' or value['target']['name'] not in EXTERNS:continue
  candidates=[str(Path(f).resolve()) for f in value['filenames'] if f.endswith('.rlib')];assert len(candidates)==1
  name=value['target']['name'];assert name not in artifacts or artifacts[name]==candidates[0];artifacts[name]=candidates[0]
 assert set(artifacts)==set(EXTERNS)
 helpers={}
 for name,src in [('reading-proof',ROOT/'docs/reports/guarded-contract-adoption/scripts/attribute_reading_environments.rs'),('metadata-replay',ROOT/'docs/reports/w48-bottom-up-batch4/scripts/metadata_replay.rs'),('projection-replay',ROOT/'docs/reports/w48-bottom-up-batch5/scripts/projection_replay.rs')]:
  output=out/name;command=['rustc','--edition=2024',src,'-C','debuginfo=0','-L','dependency='+str(workspace/'target/release/deps'),'-o',output]
  for external in EXTERNS:command+=['--extern',external+'='+artifacts[external]]
  run(name+'-build',command,ROOT,env);helpers[name]={'path':str(output),'sha256':sha(output),'source':str(src),'source_sha256':sha(src)}
 exporter=workspace/'target/debug/examples/premise_environment';assert exporter.exists();stage=next(r for r in commands if r['name']=='audit-examples');assert stage['exit']==0
 (out/'helpers.json').write_text(json.dumps({'source':source,'helpers':helpers,'environment_exporter':{'path':str(exporter),'sha256':sha(exporter),'gate_stage':stage},'exact_rlibs':{k:{'path':v,'sha256':sha(Path(v))} for k,v in artifacts.items()}},indent=2)+'\n')
 (out/'completion.json').write_text(json.dumps({'exit':0,'source':source,'binary_sha256':binary_sha,'commands_sha256':sha(out/'commands.json'),'completed':datetime.datetime.now(datetime.timezone.utc).isoformat()},indent=2)+'\n')
print('FINAL TIP FROZEN AND HELPERS BUILT',flush=True)
