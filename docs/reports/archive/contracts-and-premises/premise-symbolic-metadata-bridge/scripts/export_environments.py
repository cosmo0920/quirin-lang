"""Fresh explicit native environment exports; run only after joint gate release."""
import argparse,hashlib,json,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--helper',type=Path,required=True);p.add_argument('--side',choices=['base','tip'],required=True);p.add_argument('--routes',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--selected-routes',default='system,slice,corpus');a=p.parse_args();selected=a.selected_routes.split(',');assert selected and len(set(selected))==len(selected) and set(selected)<=set(['system','slice','corpus'])
sha=lambda b:hashlib.sha256(b).hexdigest();a.out.mkdir(parents=True,exist_ok=False);records=[]
for route in selected:
 inv=json.loads((a.routes/route/'invocation.json').read_bytes());assert inv['exit']==0 and inv['inputs_before']==inv['inputs_after']
 snapshot=Path(inv['inputs_snapshot'])
 for relative,expected in inv['inputs_before'].items():assert sha((snapshot/relative).read_bytes())==expected,('held snapshot changed since audit',route,relative)
 assert sha(Path(inv['binary']).read_bytes())==inv['binary_sha256'],'held native audit binary changed'
 manifest=snapshot/'contract/profiles/fluent-bit/../../../cycle19-resource-contracts.json';out=a.out/route;out.mkdir();command=[str(a.helper.resolve()),str(manifest),inv['source_root'],str(out/(a.side+'-environment.json'))]
 record={'route':route,'side':a.side,'command':command,'helper_sha256':sha(a.helper.read_bytes()),'manifest_sha256':sha(manifest.read_bytes()),'bound_invocation_sha256':sha((a.routes/route/'invocation.json').read_bytes()),'snapshot_inputs':inv['inputs_before']}
 with (out/(a.side+'-environment-hashes.json')).open('wb') as stdout,(out/(a.side+'-stderr.log')).open('wb') as stderr:r=subprocess.run(command,stdout=stdout,stderr=stderr)
 record['exit']=r.returncode;records.append(record);(a.out/'commands.json').write_text(json.dumps(records,indent=2)+'\n');assert r.returncode==0
 env=(out/(a.side+'-environment.json')).read_bytes();hashes=json.loads((out/(a.side+'-environment-hashes.json')).read_bytes());assert sha(env)==hashes['environment_sha256']
 sources=json.loads(env)['source_assumptions'];sidecar=json.loads((a.routes/route/'resource-diagnostics.contracts.json').read_bytes());assert sources==sidecar.get('sources',sidecar.get('contracts'))
 record['environment_sha256']=sha(env);record['source_map_matches_actual_route']=True
 for relative,expected in inv['inputs_before'].items():assert sha((snapshot/relative).read_bytes())==expected,('snapshot changed during export',route,relative)
(a.out/'commands.json').write_text(json.dumps(records,indent=2)+'\n');print(json.dumps({'side':a.side,'exports':len(records),'exit':0}))
