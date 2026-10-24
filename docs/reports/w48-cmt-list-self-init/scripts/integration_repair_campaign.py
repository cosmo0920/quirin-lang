from pathlib import Path
import json,subprocess,os,datetime,time,sys,shutil
p=Path('private/w48-cmt-member-storage');write=lambda name,x:(p/name).write_text(json.dumps(x,indent=2)+'\n');read=lambda f:json.loads(Path(f).read_text())
prior=read(p/'development-lock.json');write('development-lock-before-integration-repair.json',prior)
record={'state':'development_slot_acquired','pid':os.getpid(),'head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'acquired_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'purpose':'Repair combined-source test expectations and complete user-authorized main integration'}
write('development-lock.json',record);write('integration-repair-slot.json',record)
def wait(name,clean=False):
 while not (p/name).exists():time.sleep(1)
 value=read(p/name)
 if clean:
  assert value['head']==subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
  assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
 return value
def run(cmd):
 code=subprocess.run(cmd).returncode
 if code:sys.exit(code)
wait('integration-repair-edits-ready.json')
run(['python3',str(p/'run_step.py'),'fmt-integration-repair','cargo','fmt','--all'])
write('integration-repair-format-complete.json',{'exit':0,'at':datetime.datetime.now(datetime.timezone.utc).isoformat()})
wait('integration-repair-ready.json',clean=True)
old=read(p/'integration-gate/20261024T092516Z/gate-run.json')
head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
changed=subprocess.check_output(['git','diff','--name-only',old['head'],head],text=True).splitlines();allowed={'compiler/tests/contract_loader.rs','crates/qcore-contract/tests/production_symbolic_adoption.rs','scripts/tests/test_contract_corpus.py'}
provider=[x for x in old['steps'] if x['name'].startswith(('provider-','verify-'))]
can_reuse=len(provider)==11 and all(x['exit']==0 for x in provider) and all(name.startswith('docs/') or name in allowed for name in changed)
if can_reuse:
 run(['python3',str(p/'integration_root_gate.py')])
 new=read(p/'latest-root-repair-gate.json');assert new['exit']==0;d=Path(new['path']);g=read(d/'gate-run.json')
 reuse={'measured_source':old['head'],'applicable_source':g['head'],'reason':'Only three test expectations and documentation changed; provider inputs and all runtime/library source remain byte-identical. Successful provider commands are reused, not rerun or relabelled as fresh.','changed_paths':changed,'original_gate':str(p/'integration-gate/20261024T092516Z/gate-run.json')}
 g['provider_evidence_reuse']=reuse
 for step in provider:
  copied=dict(step,measured_source=old['head'],source_equivalent=True);g['steps'].append(copied)
  for suffix in ['.log','.exit']:
   src=p/'integration-gate/20261024T092516Z'/(step['name']+suffix)
   if src.exists():shutil.copyfile(src,d/src.name)
 (d/'provider-evidence-reuse.json').write_text(json.dumps(reuse,indent=2)+'\n');(d/'gate-run.json').write_text(json.dumps(g,indent=2)+'\n');write('latest-integration-gate.json',{'path':str(d.resolve()),'head':g['head'],'exit':0})
else:
 # A newly advanced main with changed runtime/provider inputs needs fresh gates.
 run(['python3',str(p/'integration_gate.py')])
write('integration-gate-complete.json',{'exit':0,'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'provider_reused':can_reuse})
wait('integration-matrix-ready.json',clean=True)
run(['python3',str(p/'run_step.py'),'post-matrix-python-1','python3','-m','unittest','discover','-s','scripts/tests'])
write('integration-matrix-complete.json',{'exit':0,'at':datetime.datetime.now(datetime.timezone.utc).isoformat()})
wait('integration-final-release.json')
record.update(state='released',released_at=datetime.datetime.now(datetime.timezone.utc).isoformat());write('integration-repair-slot.json',record)
