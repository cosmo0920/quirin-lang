"""After gate and six audits: fresh exports, exact metadata proof, full records.
All native helper executables are supplied explicitly by root. No cargo/build.
"""
import argparse,hashlib,json,os,subprocess,sys
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--environment-helper',type=Path,required=True);p.add_argument('--reading-helper',type=Path,required=True);p.add_argument('--tip-views',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--base-views',type=Path,default=Path('private/premise-resolution-cycle2/prepared-base'));p.add_argument('--routes',default='system,slice,corpus');a=p.parse_args();selected=a.routes.split(',');assert selected and len(set(selected))==len(selected) and set(selected)<=set(['system','slice','corpus']);a.out.mkdir(parents=True,exist_ok=False)
P=Path('private/premise-resolution-cycle2');steps=[];sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();env=dict(os.environ,QCORE_CLANG='/usr/lib/llvm-21/bin/clang')
def run(name,command,allowed=(0,)):
 r={'name':name,'command':list(map(str,command)),'cwd':str(Path.cwd()),'root_head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'environment_overrides':{'QCORE_CLANG':env['QCORE_CLANG']}}
 with (a.out/(name+'.log')).open('wb') as log:result=subprocess.run(r['command'],env=env,stdout=log,stderr=subprocess.STDOUT)
 r.update(exit=result.returncode,log=str(a.out/(name+'.log')));steps.append(r);(a.out/'commands.json').write_text(json.dumps(steps,indent=2)+'\n');assert result.returncode in allowed,r
for side,routes in [('base',a.base_views),('tip',a.tip_views)]:run('export-'+side,[sys.executable,P/'export_environments.py','--helper',a.environment_helper,'--side',side,'--routes',routes,'--out',a.out/(side+'-environments'),'--selected-routes',a.routes])
results=[]
for route in selected:
 base=a.base_views/route;tip=a.tip_views/route;known=a.out/(route+'-known.json');proof=a.out/(route+'-reading-proof.json');spec=a.out/(route+'-reading-input.json');target=a.out/route
 run(route+'-identities',[sys.executable,P/'derive_joint_identities.py',route,base,tip,a.out/'base-environments',a.out/'tip-environments',known])
 inv={s:json.loads((d/'invocation.json').read_bytes()) for s,d in [('base',base),('tip',tip)]}
 spec.write_text(json.dumps({'base_environment':str(a.out/'base-environments'/route/'base-environment.json'),'tip_environment':str(a.out/'tip-environments'/route/'tip-environment.json'),'base_diagnostics':str(base/'resource-diagnostics.json'),'tip_diagnostics':str(tip/'resource-diagnostics.json'),'base_contracts':str(base/'resource-diagnostics.contracts.json'),'tip_contracts':str(tip/'resource-diagnostics.contracts.json'),'base_inputs_root':inv['base']['inputs_snapshot'],'tip_inputs_root':inv['tip']['inputs_snapshot'],'base_build_commit':inv['base']['analyzer_source'],'tip_build_commit':inv['tip']['analyzer_source']},indent=2)+'\n')
 run(route+'-canonical-capture-proof',[a.reading_helper,spec,proof])
 run(route+'-all-fields',[sys.executable,P/'compare_records.py',route,base,tip,target,'--known',known,'--reading-proof',proof],allowed=(0,1))
 comparison=json.loads((target/'comparison.json').read_bytes())
 if comparison['all_existing_fields_equal']:
  comparison.update(all_existing_fields_equal_after_exact_time_identity_attribution=True,all_fields_retained=True,unexplained_remaining_leaf_counts=comparison['remaining_leaf_counts']);(target/'final-comparison.json').write_text(json.dumps(comparison,indent=2)+'\n')
 else:run(route+'-exact-time-identity',[sys.executable,P/'attribute_time_identity.py',route,target])
 final=json.loads((target/'final-comparison.json').read_bytes());assert final['all_existing_fields_equal_after_exact_time_identity_attribution'] and not any(final['unexplained_remaining_leaf_counts'].values());results.append(final)
(a.out/'comparison-summary.json').write_text(json.dumps({'routes':results,'environment_helper_sha256':sha(a.environment_helper),'reading_helper_sha256':sha(a.reading_helper),'all_fields_retained':True,'no_atom_inverse':True,'no_verdict_rewrite':True},indent=2)+'\n');print(json.dumps({'routes':len(results),'all_full_records_equal_after_exact_identity_proofs':True}))
