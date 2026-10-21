"""Opt-in ROOT postgate compiler; no replay or native auditor execution.
Run from the exact frozen root checkout, under SMALL after its successful gate.
"""
import argparse,hashlib,json,os,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--gate',type=Path,required=True);p.add_argument('--source',required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
P=Path(__file__).resolve().parent;out=a.out.resolve();out.mkdir(parents=True,exist_ok=False);records=[]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
gate_raw=(a.gate/'acquired.json').read_bytes();gate=json.loads(gate_raw);frozen=a.source;assert len(frozen)==40 and gate['head']==frozen
commands=json.loads((a.gate/'commands.json').read_bytes());assert commands and all(x['exit']==0 and x['source']==frozen for x in commands), 'successful complete gate required'
required={'rust','fmt','clippy','lean','provider-rust','provider-python','provider-contracts','package-build','audit-examples','release'}|{'package-verify-'+x for x in ['chunkio','libevent','librdkafka','cmetrics','ctraces']}
assert required <= {x['name'] for x in commands};assert (a.gate/'released.json').exists()
overrides={'QCORE_AUDIT_BUILD_COMMIT':frozen,'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0'};env=dict(os.environ,**overrides)
def guard():
 head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();delta=subprocess.check_output(['git','diff','--no-ext-diff','--binary','HEAD']);assert head==frozen and not delta, 'frozen root HEAD/source changed'
 return {'head':head,'tracked_delta_sha256':hashlib.sha256(delta).hexdigest()}
guard()
def run(name,command):
 before=guard()
 with (out/(name+'.stdout.log')).open('wb') as stdout,(out/(name+'.stderr.log')).open('wb') as stderr:r=subprocess.run(list(map(str,command)),env=env,stdout=stdout,stderr=stderr)
 records.append({'name':name,'command':list(map(str,command)),'exit':r.returncode,'source_before':before,'source_after':guard(),'environment_overrides':overrides});(out/'commands.json').write_text(json.dumps(records,indent=2)+'\n');assert r.returncode==0
run('cargo-artifacts',['cargo','build','--release','-p','qcore-audit','--lib','--example','premise_environment','--message-format=json','-j1'])
artifacts={};executables={}
for line in (out/'cargo-artifacts.stdout.log').read_bytes().splitlines():
 x=json.loads(line)
 if x.get('reason')!='compiler-artifact':continue
 for f in x.get('filenames',[]):
  if f.endswith('.rlib'):artifacts[x['target']['name']]=str(Path(f).resolve())
 if x.get('executable'):executables[x['target']['name']]=str(Path(x['executable']).resolve())
names=['qcore_audit','qcore_c_gateway','qcore_contract','qcore_schemas','serde_json']
command=['rustc','--edition=2024',str(P/'attribute_reading_environments.rs'),'-C','debuginfo=0','-L','dependency='+str(Path('target/release/deps').resolve()),'-o',str(out/'attribute-reading-environments')]
for name in names:command += ['--extern',name+'='+artifacts[name]]
run('reading-proof-build',command)
receipt={'source':frozen,'source_guard':guard(),'independently_held_gate_acquisition':str((a.gate/'acquired.json').resolve()),'gate_acquisition_sha256':hashlib.sha256(gate_raw).hexdigest(),'gate_commands_sha256':sha(a.gate/'commands.json'),'environment_overrides':overrides,'reading_helper':str(out/'attribute-reading-environments'),'reading_helper_sha256':sha(out/'attribute-reading-environments'),'report_source_sha256':{name:sha(P/name) for name in ['attribute_reading_environments.rs','source_bridge.rs','compile_helpers.py']},'environment_helper':executables['premise_environment'],'environment_helper_sha256':sha(executables['premise_environment']),'environment_source_sha256':sha('crates/qcore-audit/examples/premise_environment.rs'),'exact_rlibs':{n:{'path':artifacts[n],'sha256':sha(artifacts[n])} for n in names},'checker_execution':'compile only; no canonical export, reading replay or native auditor'}
(out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
