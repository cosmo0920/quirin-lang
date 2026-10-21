import datetime,fcntl,hashlib,json,os,subprocess,sys
from pathlib import Path
P=Path('private/premise-resolution-cycle2');previous=P/'comparison-configured-1';out=P/'comparison-configured-continuation-2';out.mkdir(exist_ok=False);steps=[];head='62eebba95c51917c0fe841fbc23e2bf0d0e7e465';env=dict(os.environ,QCORE_CLANG='/usr/lib/llvm-21/bin/clang');sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def run(name,command,allowed=(0,)):
 assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==head
 with (out/(name+'.log')).open('wb') as log:r=subprocess.run(list(map(str,command)),stdout=log,stderr=subprocess.STDOUT,env=env)
 steps.append({'name':name,'command':list(map(str,command)),'exit':r.returncode,'log':str(out/(name+'.log')),'source':head});(out/'commands.json').write_text(json.dumps(steps,indent=2)+'\n');assert r.returncode in allowed
with open('/tmp/qcore-c-cycle2-small-compile.lock','a') as lock:
 fcntl.flock(lock,fcntl.LOCK_EX);(out/'acquired.json').write_text(json.dumps({'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source':head,'concurrency':'root corpus16 PRIMARY; serial single-helper SMALL comparator'})+'\n')
 # The completed successful native system proof is used exactly once, without
 # another export/replay. The comparator rechecks its held diagnostic hashes.
 proof=previous/'system-reading-proof.json';known=previous/'system-known.json'
 (out/'reused-proof.json').write_text(json.dumps({'native_system_proof':str(proof),'sha256':sha(proof),'known':str(known),'known_sha256':sha(known),'completed_prior_commands':str(previous/'commands.json'),'reason':'prior Python assertion happened after successful native proof; no repeated native campaign'},indent=2)+'\n')
 for route in ['system','slice']:
  base=P/'prepared-base'/route;tip=P/'prepared-tip'/route
  if route=='slice':
   known=out/'slice-known.json';proof=out/'slice-reading-proof.json';spec=out/'slice-reading-input.json'
   run('slice-identities',[sys.executable,P/'derive_joint_identities.py',route,base,tip,previous/'base-environments',previous/'tip-environments',known])
   inv={s:json.loads((d/'invocation.json').read_bytes()) for s,d in [('base',base),('tip',tip)]}
   spec.write_text(json.dumps({'base_environment':str(previous/'base-environments'/route/'base-environment.json'),'tip_environment':str(previous/'tip-environments'/route/'tip-environment.json'),'base_diagnostics':str(base/'resource-diagnostics.json'),'tip_diagnostics':str(tip/'resource-diagnostics.json'),'base_contracts':str(base/'resource-diagnostics.contracts.json'),'tip_contracts':str(tip/'resource-diagnostics.contracts.json'),'base_inputs_root':inv['base']['inputs_snapshot'],'tip_inputs_root':inv['tip']['inputs_snapshot'],'base_build_commit':inv['base']['analyzer_source'],'tip_build_commit':inv['tip']['analyzer_source']},indent=2)+'\n')
   run('slice-canonical-capture-proof',[P/'comparator-helpers/attribute-reading-environments',spec,proof])
  target=out/route;run(route+'-all-fields',[sys.executable,P/'compare_records.py',route,base,tip,target,'--known',known,'--reading-proof',proof],allowed=(0,1))
  comparison=json.loads((target/'comparison.json').read_bytes())
  if comparison['all_existing_fields_equal']:
   comparison.update(all_existing_fields_equal_after_exact_time_identity_attribution=True,all_fields_retained=True,unexplained_remaining_leaf_counts=comparison['remaining_leaf_counts']);(target/'final-comparison.json').write_text(json.dumps(comparison,indent=2)+'\n')
  else:run(route+'-exact-time-identity',[sys.executable,P/'attribute_time_identity.py',route,target])
  final=json.loads((target/'final-comparison.json').read_bytes());assert final['all_existing_fields_equal_after_exact_time_identity_attribution'] and not any(final['unexplained_remaining_leaf_counts'].values())
(out/'released.json').write_text(json.dumps({'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source':head})+'\n');print('configured full comparisons complete0')
