from pathlib import Path
import json,hashlib,subprocess,gzip,shutil,re
root=Path.cwd();p=root/'private/w48-cmt-member-storage';r=root/'docs/reports/w48-cmt-list-self-init';out=r/'integration'
read=lambda f:json.loads(Path(f).read_text());write=lambda f,x:Path(f).write_text(json.dumps(x,indent=2)+'\n');sha=lambda data:hashlib.sha256(data).hexdigest()
assert read(p/'integration-matrix-complete.json')['exit']==0
post=read(p/'steps/post-matrix-python-1.json');assert post['exit']==0 and not post['dirty']
head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();assert head==post['head']
matrix=root/'docs/reviews/r0_3/acceptance_matrix.json';committed=subprocess.check_output(['git','show','HEAD:docs/reviews/r0_3/acceptance_matrix.json']);assert matrix.read_bytes()==committed
log=Path(post['log']).read_bytes();assert re.search(rb'Ran 374 tests',log) and b'\nOK\n' in log
write(out/'post-matrix-confirmation.json',{'head':head,'matrix_sha256':sha(committed),'command_receipt':post,'tests':374,'exit':0,'log_sha256':sha(log),'log':'post-matrix-python.log.gz'})
(out/'post-matrix-python.log.gz').write_bytes(gzip.compress(log,mtime=0))
g=read(out/'standing-gate.json');changes=subprocess.check_output(['git','diff','--name-only',g['head'],head],text=True).splitlines();assert all(name.startswith('docs/') for name in changes),changes
write(out/'source-equality.json',{'measured_root_source':g['head'],'post_matrix_head':head,'non_documentation_changes':[],'documentation_changes':changes,'critical_prefix_fixture_changes':subprocess.check_output(['git','diff','--name-only',g['head'],head,'--','docs/reports/w48-cmt-list-self-init/integration/rule37-memory-before-prefix.txt','docs/reports/w48-cmt-list-self-init/integration/rule37-memory-after-prefix.txt'],text=True).splitlines()})
assert not read(out/'source-equality.json')['critical_prefix_fixture_changes']
a=read(r/'validation-attempts.json');steps=[]
for f in sorted((p/'steps').glob('*.json')):
 x=read(f);path=Path(x['log']);x['test_summaries']=re.findall(r'^test result: .*$',path.read_text(errors='replace'),re.M)
 if f.stem=='accounting-1':x['native_calculation_exit']=0;x['combined_integration_handoff_exit']=1
 steps.append(x);(r/'validation-logs'/(path.name+'.gz')).write_bytes(gzip.compress(path.read_bytes(),mtime=0))
a['focused_build_measurement_attribution_steps']=steps
a['native_accounting_handoff']={'native_calculation_exit':0,'complete_raw_native_comparison':'native-comparison.json','first_combined_handoff_exit':1,'reason':'The optional post-native integration attempt found two Rust and one Python expectation failures. Native accounting had already completed successfully and its raw results were preserved.','repaired_root_exit':0,'provider_reused':bool(g.get('provider_evidence_reuse'))}
a['integration_attempts']=[{'source':'389090b6755b43045c8838989c91adce33e9aabe','exit':1,'rust':{'passed':8637,'failed':2,'ignored':15,'units':549,'summaries':562},'python':{'tests':374,'failed':1},'lean':{'audits':370,'exit':0},'provider_commands_exit':0,'retained':'integration/attempts/20261024T092516Z'},{'source':g['head'],'root_exit':0,'root_rust':g['rust']['totals'],'lean':g['lean_audits'],'python':g['python_tests'],'provider_origin':g.get('provider_evidence_reuse',{'measured_source':g['head'],'fresh':True}),'receipt':'integration/standing-gate.json'}];write(r/'validation-attempts.json',a)
rec=read(out/'reconciliation.json');rec['remaining']=['Advance main and push configured origin; actual released-lock receipt follows.'];rec['items'] += [{'card':'both CMetrics increments','item':'report-index','decision':'applied','verify':[{'file':'docs/reports/INDEX.md','contains':'w48-cmt-list-self-init/README.md'},{'file':'docs/reports/INDEX.md','contains':'w48-bu-cmt-map-metric-get/README.md'}]},{'card':'both CMetrics increments','item':'committed-matrix-and-python','decision':'applied','verify':[{'file':'docs/reviews/r0_3/acceptance_matrix.json','contains':'W48-CMT-TRACKED-INTERIOR-SELF-INIT'},{'file':'docs/reports/w48-cmt-list-self-init/integration/post-matrix-confirmation.json','contains':'"exit": 0'}]}]
for item in rec['items']:
 assert all(v['contains'] in (root/v['file']).read_text() for v in item['verify']),item
write(out/'reconciliation.json',rec)
f=r/'README.md';s=f.read_text();s+='\nFinal committed-matrix Python acceptance passes all 374 tests at `'+head+'`; the [confirmation](integration/post-matrix-confirmation.json) binds the exact committed matrix hash and command. Only report/matrix/status documentation differs from the measured root source, and the pinned historical transition fixtures remain unchanged. Main advancement and pushing use these validated source bytes.\n';f.write_text(s)
shutil.copyfile(p/'finalize_integration.py',r/'scripts/finalize_integration.py')
write(r/'artifact-inventory.json',{str(f.relative_to(r)):{'sha256':sha(f.read_bytes()),'bytes':f.stat().st_size} for f in sorted(r.rglob('*')) if f.is_file() and f.name!='artifact-inventory.json'})
print('Final confirmation and complete attempt history prepared; release and push still required.')
