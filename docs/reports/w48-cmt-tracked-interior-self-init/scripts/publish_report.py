from pathlib import Path
import json,gzip,shutil,re,hashlib
root=Path.cwd();p=root/'private/w48-cmt-tracked-interior';r=root/'docs/reports/w48-cmt-tracked-interior-self-init'
read=lambda f:json.loads(Path(f).read_bytes())
write=lambda f,v:Path(f).write_text(json.dumps(v,indent=2)+'\n')
sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
native=read(p/'native-comparison.json');assert native['unexplained_leaves']==0
latest=read(p/'latest-gate.json');assert latest['exit']==0
g=Path(latest['path']);gate=read(g/'gate-run.json');assert gate['exit']==0
summary=read(g/'rust-binaries/summary.json')
gate['rust']=summary;gate['lean_audits']=int(re.search(r'Passed (\d+) Lean audits',(g/'lean-audits.log').read_text()).group(1));gate['python_tests']=int(re.search(r'Ran (\d+) tests',(g/'python-tests.log').read_text()).group(1))
write(r/'standing-gate.json',gate)
for name in ['clang-version.txt','clang-version.exit']:shutil.copyfile(g/name,r/name)
for name in ['native-comparison.json','base-binary.json','tip-binary.json','base-build.json','base-submodules.json','semantic-freeze.json','development-lock.json']:shutil.copyfile(p/name,r/name)
shutil.copyfile(p/'inputs/manifest.json',r/'frozen-input-manifest.json')
for name in ['qv-correspondence.json','contract-model-input-delta.json','reading-contracts-reuse.json','production-targets.json','api-model-pins.json','api-source-validation.json','semantic-review-reconstruction.json']:
 shutil.copyfile(p/name,r/name)
shutil.copytree(p/'comparisons',r/'artifacts',dirs_exist_ok=True)
(r/'planner').mkdir(exist_ok=True);
for side in ['base','tip']:(r/'planner'/(side+'.json.gz')).write_bytes(gzip.compress((p/(side+'-plan.json')).read_bytes(),mtime=0))
write(r/'planner/comparison.json',{'exact_bytes_equal':native['planner_exactly_equal'],'sha256':native['planner_sha256'],'getter_closure_gain':0,'base_plan':{k:v for k,v in read(p/'base-plan.json').items() if k!='rows'},'tip_plan':{k:v for k,v in read(p/'tip-plan.json').items() if k!='rows'}})
(r/'replays').mkdir(exist_ok=True)
for f in (p/'replay-v3').iterdir():
 if f.is_file() and f.suffix in ['.json','.log'] or f.name=='exit':
  data=f.read_bytes();target=r/'replays'/f.name
  if len(data)>100000:target=target.with_name(target.name+'.gz');data=gzip.compress(data,mtime=0)
  target.write_bytes(data)
for route in ['corpus','system','slice']:
 target=r/'replays/time'/route;target.mkdir(parents=True,exist_ok=True)
 for f in (p/'replay-v3/time'/route).iterdir():
  if f.is_file() and not f.is_symlink():shutil.copyfile(f,target/f.name)
for attempt in ['replay-v1','replay-v2']:
 target=r/'validation-logs'/attempt;target.mkdir(parents=True,exist_ok=True)
 for f in (p/attempt).rglob('*'):
  if f.is_file() and not f.is_symlink() and f.suffix in ['.log','.json']:
   name=f.relative_to(p/attempt);destination=target/name;destination.parent.mkdir(parents=True,exist_ok=True);destination.write_bytes(f.read_bytes())
(r/'scripts').mkdir(exist_ok=True)
for name in ['measure.py','base_campaign.py','retain_compare.py','rebind_readings.rs','projection_replay.rs','post_attribution.py','time_identity.py','attribute_compare.py','finish_accounting.py','run_standing_gate.py','run_step.py','tip_campaign.py','provider_gate.py','freeze_and_tip.py','production_targets.py','write_report.py','publish_report.py','provider_api_gate.py','prepare_api.py','focus_api_lean.py','review_semantic.py','time_identity_v1.py','time_identity_v2.py','api_source_validation.py','seal_publication.py','apply_current_docs.py','finalize_source_reports.py']:
 shutil.copyfile(p/name,r/'scripts'/name)
attempts=[]
for d in sorted((p/'gate').iterdir()):
 rec=read(d/'gate-run.json');rec['interruption']=read(d/'interruption.json') if (d/'interruption.json').exists() else None
 attempts.append(rec)
logs=r/'validation-logs';logs.mkdir(exist_ok=True)
for d in sorted((p/'gate').iterdir()):
 target=logs/d.name;target.mkdir(exist_ok=True)
 for f in d.iterdir():
  if f.is_file() and f.suffix in ['.log','.txt','.exit','.json']:(target/(f.name+'.gz')).write_bytes(gzip.compress(f.read_bytes(),mtime=0))
 summary_file=d/'rust-binaries/summary.json'
 if summary_file.exists():
  for unit in read(summary_file)['units']:
   if unit['ok']:continue
   for key in ['stdout_log','stderr_log']:
    f=Path(unit[key]);(target/(f.name+'.gz')).write_bytes(gzip.compress(f.read_bytes(),mtime=0))
steps=[]
for f in sorted((p/'steps').glob('*.json')):
 rec=read(f);log=Path(rec['log']);rec['test_summaries']=re.findall(r'^test result: .*$',log.read_text(errors='replace'),re.M);steps.append(rec)
 (logs/(log.name+'.gz')).write_bytes(gzip.compress(log.read_bytes(),mtime=0))
write(r/'validation-attempts.json',{'standing_gate_attempts':attempts,'focused_build_measurement_attribution_steps':steps,'no_failed_attempt_hidden':True,'premeasurement_queue_attempts':[read(p/'base-queue-attempt1.json')] if (p/'base-queue-attempt1.json').exists() else []})

shutil.copytree(p/'provider-gate',r/'initializer-provider-gate',dirs_exist_ok=True)
shutil.copytree(p/'provider-api-gate',r/'provider-gate',dirs_exist_ok=True)
write(r/'artifact-inventory.json',{str(f.relative_to(r)):{'sha256':sha(f),'bytes':f.stat().st_size} for f in sorted(r.rglob('*')) if f.is_file() and f.name!='artifact-inventory.json'})
print(json.dumps({'gate':str(g),'rust':summary.get('totals'),'lean_audits':gate['lean_audits'],'python':gate['python_tests'],'native':{k:v for k,v in native.items() if k!='routes'}},indent=2))
