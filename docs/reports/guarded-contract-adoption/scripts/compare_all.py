#!/usr/bin/env python3
"""Run complete private source, canonical, metadata and projection attribution."""
import argparse,datetime,fcntl,hashlib,json,os,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];REPORT=ROOT/'docs/reports/guarded-contract-adoption/scripts'
p=argparse.ArgumentParser();p.add_argument('--base',type=Path,required=True);p.add_argument('--tip',type=Path,required=True);p.add_argument('--base-build',type=Path,required=True);p.add_argument('--tip-preparation',type=Path,required=True);p.add_argument('--run',required=True);a=p.parse_args()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();read=lambda p:json.loads(p.read_bytes())
out=ROOT/'private/guarded-contract-adoption'/a.run;out.mkdir(parents=True,exist_ok=False);helpers=read(a.tip_preparation/'helpers.json');records=[];head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
base_build=read(a.base_build);tip_build=read(a.tip_preparation/'binary-receipt.json')
for native in [a.base,a.tip]:assert read(native/'completion.json')['exit']==0
assert read(a.tip_preparation/'completion.json')['exit']==0
for v in list(helpers['helpers'].values())+[helpers['environment_exporter']]:assert sha(Path(v['path']))==v['sha256']
def guard():
 assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()==head
 assert not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True)
def run(name,command,env=None):
 guard();start=datetime.datetime.now(datetime.timezone.utc).isoformat()
 with (out/(name+'.stdout.log')).open('wb') as stdout,(out/(name+'.stderr.log')).open('wb') as stderr:r=subprocess.run(list(map(str,command)),cwd=ROOT,env=env,stdout=stdout,stderr=stderr)
 records.append({'name':name,'command':list(map(str,command)),'cwd':str(ROOT),'exit':r.returncode,'started':start,'ended':datetime.datetime.now(datetime.timezone.utc).isoformat()});(out/'commands.json').write_text(json.dumps(records,indent=2)+'\n');guard();assert r.returncode==0,name
with open('/tmp/qcore-c-card-validation-20261022.lock','a') as lock:
 print('WAITING FOR COMPLETE COMPARISON LOCK',flush=True);fcntl.flock(lock,fcntl.LOCK_EX);guard()
 for route in ['system','slice','corpus']:
  raw=out/route;raw.mkdir()
  stage=raw/'stage';envroot=out/'environments'
  run(route+'-stage',[sys.executable,REPORT/'stage_inputs.py',route,a.base/route,a.tip/route,stage])
  run(route+'-raw',[sys.executable,REPORT/'retain_compare.py',route,a.base/route,a.tip/route,raw/'raw'])
  run(route+'-runtime-invocations',[sys.executable,REPORT/'compare_invocations.py',route,stage/'base',stage/'tip',raw/'runtime-invocations.json'])
  inv={side:read(stage/side/'invocation.json') for side in ['base','tip']}
  for side in ['base','tip']:
   run(route+'-'+side+'-export',[sys.executable,ROOT/'docs/reports/w48-bottom-up-batch5/scripts/export_environment.py','--invocation',stage/side/'invocation.json','--helper',helpers['environment_exporter']['path'],'--side',side,'--out',envroot/side/route])
   run(route+'-'+side+'-metadata',[helpers['helpers']['metadata-replay']['path'],stage/side/'resource-diagnostics.json',stage/side/'resource-diagnostics.contracts.json',envroot/side/route/(side+'-environment.json'),raw/('metadata-'+side+'.json')])
  spec={side+'_'+kind:str(path) for side in ['base','tip'] for kind,path in [('environment',envroot/side/route/(side+'-environment.json')),('diagnostics',stage/side/'resource-diagnostics.json'),('contracts',stage/side/'resource-diagnostics.contracts.json')]}
  spec.update(base_inputs_root=inv['base']['inputs_snapshot'],tip_inputs_root=inv['tip']['inputs_snapshot'],base_build_commit=base_build['source'],tip_build_commit=tip_build['source'],guarded_metadata_plan=str(ROOT/'docs/reports/guarded-contract-adoption/adoption-plan.json'))
  specfile=raw/'reading-spec.json';specfile.write_text(json.dumps(spec,indent=2)+'\n')
  env=dict(os.environ,QCORE_CLANG='/usr/bin/clang' if route=='corpus' else '/usr/lib/llvm-21/bin/clang')
  run(route+'-reading',[helpers['helpers']['reading-proof']['path'],specfile,raw/'reading-proof.json'],env)
  run(route+'-identities',[sys.executable,REPORT/'derive_identities.py',route,stage/'base',stage/'tip',envroot/'base',envroot/'tip',raw/'known-identities.json','--reading-proof',raw/'reading-proof.json'])
  # Both new invocations have actual full before/after manifests. No old
  # initial-configured receipt is read or used by this selected branch.
  unused=raw/'not-borrowed.json';unused.write_text('{}\n')
  run(route+'-comparison',[sys.executable,REPORT/'compare_records.py',route,stage/'base',stage/'tip',raw/'comparison','--known',raw/'known-identities.json','--reading-proof',raw/'reading-proof.json','--base-build-receipt',a.base_build,'--tip-build-receipt',a.tip_preparation/'binary-receipt.json','--initial-base-receipts',unused])
  for side in ['base','tip']:
   snapshot=Path(inv[side]['inputs_snapshot']);source=Path(inv[side]['source_root'])
   projection={'diagnostics':str(stage/side/'resource-diagnostics.json'),'contracts':str(stage/side/'resource-diagnostics.contracts.json'),'null':str(stage/side/(route+'-null.jsonl')),'null_summary':str(stage/side/(route+'-null.summary.json')),'outer':str(stage/side/(route+'-outer.json')),'environment':str(envroot/side/route/(side+'-environment.json')),'manifest':str(snapshot/'contract/profiles/fluent-bit/../../../cycle19-resource-contracts.json'),'database':str(source/'build/compile_commands.json') if route=='corpus' else inv[side]['compile_database'],'route':'pinned' if route=='corpus' else 'configured','source_root':str(source)}
   path=raw/(side+'-projection-spec.json');path.write_text(json.dumps(projection,indent=2)+'\n');run(route+'-'+side+'-projection',[helpers['helpers']['projection-replay']['path'],path,raw/('projection-'+side+'.json')],env)
  result=read(raw/'comparison/comparison.json')
  if not result['all_existing_fields_equal']:
   run(route+'-time-preimages',[sys.executable,ROOT/'docs/reports/premise-symbolic-metadata-bridge/scripts/attribute_time_identity.py',route,raw/'comparison'])
   final=read(raw/'comparison/final-comparison.json');assert all(v==0 for v in final['unexplained_remaining_leaf_counts'].values())
  print('COMPARED',route,flush=True)
 guard();(out/'completion.json').write_text(json.dumps({'exit':0,'head':head,'command_count':len(records),'commands_sha256':sha(out/'commands.json'),'completed':datetime.datetime.now(datetime.timezone.utc).isoformat(),'claim':'All complete source, null/static/runtime/classification/standing/canonical payload differences independently reconstructed; raw phase memory/job observations retained separately, no source truth promotion.'},indent=2)+'\n')
