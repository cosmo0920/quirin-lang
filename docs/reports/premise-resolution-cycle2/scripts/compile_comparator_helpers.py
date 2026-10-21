"""Root postgate only: compile explicit helper against Cargo-reported artifacts."""
import argparse,hashlib,json,os,subprocess
from pathlib import Path
P=Path('private/premise-resolution-cycle2');parser=argparse.ArgumentParser();parser.add_argument('--out',type=Path,default=P/'comparator-helpers');args=parser.parse_args();out=args.out;out.mkdir(exist_ok=False);records=[];env=dict(os.environ,CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0');sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
gate_path=P/'joint-gate-2/acquired.json';gate_raw=gate_path.read_bytes();frozen=json.loads(gate_raw)['head']
env['QCORE_AUDIT_BUILD_COMMIT']=frozen
def guard():
 head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();delta=subprocess.check_output(['git','diff','--no-ext-diff','--binary','HEAD']);assert head==frozen and not delta,'root frozen HEAD/source changed';return {'head':head,'tracked_delta_sha256':hashlib.sha256(delta).hexdigest()}
guard()
def run(name,command):
 before=guard()
 with (out/(name+'.stdout.log')).open('wb') as stdout,(out/(name+'.stderr.log')).open('wb') as stderr:r=subprocess.run(command,env=env,stdout=stdout,stderr=stderr)
 records.append({'name':name,'command':command,'exit':r.returncode,'source_before':before,'source_after':guard(),'environment_overrides':{'QCORE_AUDIT_BUILD_COMMIT':frozen,'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0'}});(out/'commands.json').write_text(json.dumps(records,indent=2)+'\n');assert r.returncode==0
run('cargo-artifacts',['cargo','build','--release','-p','qcore-audit','--lib','--example','premise_environment','--message-format=json','-j1'])
artifacts={};executables={}
for line in (out/'cargo-artifacts.stdout.log').read_bytes().splitlines():
 x=json.loads(line)
 if x.get('reason')!='compiler-artifact':continue
 for f in x.get('filenames',[]):
  if f.endswith('.rlib'):artifacts[x['target']['name']]=str(Path(f).resolve())
 if x.get('executable'):executables[x['target']['name']]=str(Path(x['executable']).resolve())
command=['rustc','--edition=2024',str(P/'attribute_reading_environments.rs'),'-C','debuginfo=0','-L','dependency='+str(Path('target/release/deps').resolve()),'-o',str(out/'attribute-reading-environments')]
for name in ['qcore_audit','qcore_c_gateway','qcore_contract','qcore_schemas','serde_json']:command+=['--extern',name+'='+artifacts[name]]
run('reading-proof-build',command)
receipt={'source':frozen,'source_guard':guard(),'independently_held_gate_acquisition':str(gate_path),'gate_acquisition_sha256':hashlib.sha256(gate_raw).hexdigest(),'environment_overrides':{'QCORE_AUDIT_BUILD_COMMIT':frozen,'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0'},'reading_helper':str((out/'attribute-reading-environments').resolve()),'reading_helper_sha256':sha(out/'attribute-reading-environments'),'reading_source_sha256':sha(P/'attribute_reading_environments.rs'),'environment_helper':executables['premise_environment'],'environment_helper_sha256':sha(executables['premise_environment']),'environment_source_sha256':sha('crates/qcore-audit/examples/premise_environment.rs'),'exact_rlibs':{n:{'path':artifacts[n],'sha256':sha(artifacts[n])} for n in ['qcore_audit','qcore_c_gateway','qcore_contract','qcore_schemas','serde_json']},'checker_execution':'no environment export or receipt replay during compile; separate explicit runner required'};(out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
