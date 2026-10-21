#!/usr/bin/env python3
"""After native completion, serialize exact source/canonical/projection checks."""
import argparse,datetime,fcntl,hashlib,json,os,subprocess,sys
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('output',type=Path);a=p.parse_args();root=Path.cwd();private=root/'private/w48-batch5';native=Path('/home/cosmo/.codex/worktrees/symbolic-cycle3-integration/qcore-c/private/symbolic-decomposition-cycle3/postgate-1');out=a.output.resolve();assert '/private/' in str(out);out.mkdir(parents=True,exist_ok=False);sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();now=lambda:datetime.datetime.now(datetime.timezone.utc).isoformat();records=[]
with open('/tmp/qcore-c-card-validation-20261022.lock','a') as heavy,open('/tmp/qcore-c-cycle2-small-compile.lock','a') as small:
 print('Waiting for completed TIP and attribution locks',flush=True);fcntl.flock(heavy,fcntl.LOCK_EX);fcntl.flock(small,fcntl.LOCK_EX);print('Attribution locks acquired',flush=True)
 head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
 def guard():
  assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==head
  assert not subprocess.check_output(['git','status','--porcelain'],text=True)
 def run(name,command,env=None):
  guard();record={'name':name,'command':list(map(str,command)),'cwd':str(root),'started':now()}
  with (out/(name+'.stdout.log')).open('wb') as stdout,(out/(name+'.stderr.log')).open('wb') as stderr:r=subprocess.run(command,stdout=stdout,stderr=stderr,env=env)
  record.update(exit=r.returncode,ended=now());records.append(record);(out/'commands.json').write_text(json.dumps(records,indent=2)+'\n');guard();assert r.returncode==0,name
 completion=json.loads((native/'completion.json').read_bytes());assert completion['exit']==0
 helpers=json.loads((native/'helpers.json').read_bytes());base_build=json.loads((root/'docs/reports/w48-bottom-up-batch5/base-binary.json').read_bytes());tip_build=json.loads((native/'binary-receipt.json').read_bytes());assert tip_build['source']==helpers['analyzer_source']
 (out/'acquired.json').write_text(json.dumps({'head':head,'acquired':now(),'native_completion_sha256':sha(native/'completion.json'),'helper_receipt_sha256':sha(native/'helpers.json')},indent=2)+'\n')
 for data in helpers['helpers'].values():assert sha(data['path'])==data['sha256']
 exporter=helpers['environment_exporter'];assert sha(exporter['path'])==exporter['sha256']
 run('export-tip-corpus',['python3','docs/reports/w48-bottom-up-batch5/scripts/export_environment.py','--invocation',native/'corpus/tip/invocation.json','--helper',exporter['path'],'--side','tip','--out',private/'environments/tip/corpus'])
 run('prepare-corpus',['python3','docs/reports/w48-bottom-up-batch5/scripts/prep_comparison.py','corpus'])
 command=['rustc','--edition=2024','docs/reports/w48-bottom-up-batch5/scripts/projection_replay.rs','-C','debuginfo=0','-L','dependency='+str(native.parents[2]/'target/release/deps'),'-o',str(out/'projection-replay')]
 # native=ROOT/private/report/postgate-1, so ROOT is parents[2].
 for name in ['qcore_audit','qcore_c_gateway','qcore_contract','qcore_schemas','serde_json']:
  artifact=helpers['exact_rlibs'][name];assert sha(artifact['path'])==artifact['sha256'];command+=['--extern',name+'='+artifact['path']]
 run('projection-build',command)
 for route in ['system','slice','corpus']:
  stage=private/'analysis-inputs'/route;result=private/'comparisons'/route
  inv={side:json.loads((stage/side/'invocation.json').read_bytes()) for side in ['base','tip']}
  spec={side+'_'+kind:str(path) for side in ['base','tip'] for kind,path in [('environment',private/'environments'/side/route/(side+'-environment.json')),('diagnostics',stage/side/'resource-diagnostics.json'),('contracts',stage/side/'resource-diagnostics.contracts.json')]}
  spec.update(base_inputs_root=inv['base']['inputs_snapshot'],tip_inputs_root=inv['tip']['inputs_snapshot'],base_build_commit=base_build['build_head'],tip_build_commit=tip_build['source'])
  spec_path=out/(route+'-reading-spec.json');spec_path.write_text(json.dumps(spec,indent=2)+'\n');env=dict(os.environ,QCORE_CLANG='/usr/lib/llvm-21/bin/clang')
  run(route+'-reading',[helpers['helpers']['reading-proof']['path'],spec_path,result/'reading-proof.json'],env)
  run(route+'-identities',['python3','docs/reports/w48-bottom-up-batch5/scripts/compare_attributed.py',route,stage/'base',stage/'tip',result/'identity-review','--known',result/'known-identities.json','--reading-proof',result/'reading-proof.json','--base-build-receipt',root/'docs/reports/w48-bottom-up-batch5/base-binary.json','--tip-build-receipt',native/'binary-receipt.json','--initial-base-receipts',root/'docs/reports/w48-bottom-up-batch5/baseline-receipts.json'])
  for side in ['base','tip']:
   invocation=inv[side];snapshot=Path(invocation['inputs_snapshot']);source=invocation.get('source_root')
   if source is None:source=invocation['command'][invocation['command'].index('--checkout')+1]
   projection_spec={'diagnostics':str(stage/side/'resource-diagnostics.json'),'contracts':str(stage/side/'resource-diagnostics.contracts.json'),'null':str(stage/side/(route+'-null.jsonl')),'null_summary':str(stage/side/(route+'-null.summary.json')),'outer':str(stage/side/(route+'-outer.json')),'environment':str(private/'environments'/side/route/(side+'-environment.json')),'manifest':str(snapshot/'contract/profiles/fluent-bit/../../../cycle19-resource-contracts.json'),'database':str(Path(source)/'build/compile_commands.json') if route=='corpus' else invocation['compile_database'],'route':'pinned' if route=='corpus' else 'configured','source_root':source}
   projection_path=out/(route+'-'+side+'-projection-spec.json');projection_path.write_text(json.dumps(projection_spec,indent=2)+'\n')
   env=dict(os.environ,QCORE_CLANG='/usr/bin/clang' if route=='corpus' else '/usr/lib/llvm-21/bin/clang')
   run(route+'-'+side+'-projection',[out/'projection-replay',projection_path,result/('projection-'+side+'.json')],env)
 guard();(out/'completion.json').write_text(json.dumps({'exit':0,'head':head,'completed':now(),'command_count':len(records),'commands_sha256':sha(out/'commands.json'),'scope':'Exact producer/canonical identity checks completed. Every semantic remainder still requires independently recorded source review.'},indent=2)+'\n')
print('Attribution locks released',flush=True)
