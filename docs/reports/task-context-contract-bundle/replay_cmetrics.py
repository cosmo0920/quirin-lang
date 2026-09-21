#!/usr/bin/env python3
"""Replay the existing pinned CMetrics audit with legacy and bundle selection.
Requires its already-prepared Clang checkout/build and package. Writes only to
an explicit fresh private directory. Changes no upstream or historical artifact.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
ROOT=Path(__file__).resolve().parents[3]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--package',type=Path,help='explicit independently verified CMetrics executable package');args=p.parse_args()
out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
profile=ROOT/'contract/profiles/cmetrics-counter'
audit=json.loads((profile/'audit.json').read_text())
for key in ['profile_directory','checkout','compile_database']:
 audit[key]=str((profile/audit[key]).resolve())
audit['report']=str(out/'report.json')
audit['source_contracts']['configuration']=str(out/'selection.json')
audit['source_contracts']['source_rules']=str((profile/audit['source_contracts']['source_rules']).resolve())
(out/'audit.json').write_text(json.dumps(audit,indent=2)+'\n')
config=json.loads((profile/'source-contracts.json').read_text())
config['packages']=[str(args.package.resolve())] if args.package else [str((profile/path).resolve()) for path in config['packages']]
config['bindings'][0]['contract']['manifest']=str((profile/config['bindings'][0]['contract']['manifest']).resolve())
legacy=json.loads(json.dumps(config));legacy['schema_version']=1
legacy['bindings'][0]['contract']=str(ROOT/'providers/qcore-provider-cmetrics/contracts/cmetrics/counter_reset.qcore')
facts=[]
for name,selection in [('legacy',legacy),('bundle',config)]:
 (out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
 shutil.copyfile(out/'selection.json',out/(name+'-selection.json'))
 command=[str(ROOT/'target/debug/quirin-audit'),str(out/'audit.json')]
 with (out/(name+'.log')).open('w') as log:
  code=subprocess.call(command,stdout=log,stderr=subprocess.STDOUT,cwd=ROOT)
 (out/(name+'.exit')).write_text(str(code)+'\n')
 row={'mode':name,'command':command,'exit':code,'artifacts':{}}
 for path in sorted(out.glob('report*.json')):
  data=path.read_bytes();row['artifacts'][path.name]=hashlib.sha256(data).hexdigest()
  shutil.copyfile(path,out/(name+'-'+path.name))
 facts.append(row)
 (out/'facts.json').write_text(json.dumps(facts,indent=2)+'\n')
 if code: raise SystemExit(code)
assert facts[0]['artifacts']['report.json'] == facts[1]['artifacts']['report.json']
left=json.loads((out/'legacy-report.assumptions.json').read_text())
right=json.loads((out/'bundle-report.assumptions.json').read_text())
changes=[]
def differences(a,b,path=''):
 if a == b: return
 if isinstance(a,dict) and isinstance(b,dict):
  assert a.keys() == b.keys(), path
  for key in sorted(a): differences(a[key],b[key],path+'/'+key)
 elif isinstance(a,list) and isinstance(b,list):
  assert len(a)==len(b), path
  for index,(x,y) in enumerate(zip(a,b)): differences(x,y,path+'/'+str(index))
 else: changes.append({'path':path,'before':a,'after':b})
differences(left,right)
assert {change['path'] for change in changes} == {
 '/compile_tasks/0/identity_sha256',
 '/compile_tasks/0/identity/dependencies/source_contract_configuration'
}, changes
for name,side in [('legacy',left),('bundle',right)]:
 assert len(side['compile_tasks']) == 1
 config_hash=hashlib.sha256((out/(name+'-selection.json')).read_bytes()).hexdigest()
 assert side['compile_tasks'][0]['identity']['dependencies']['source_contract_configuration']==config_hash
result={'runs':facts,'attributed_sidecar_changes':changes,
 'boundary':'Report bytes equal. Exact configuration dependency hashes rederived; the enclosing identity digest is emitted by CompileTaskIdentity::digest. All other sidecar fields equal. No production Safe promotion.'}
(out/'parity.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
