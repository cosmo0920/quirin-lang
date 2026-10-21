#!/usr/bin/env python3
"""Six fresh own executions, same exact inputs/cwd; hold shared lock outside."""
import hashlib,json,subprocess,time
from pathlib import Path
repo=Path(__file__).resolve().parents[4];private=repo/'private/libc-reading-site-evidence'
base=Path('/home/cosmo/Documents/GitHub/qcore-c/private/premise-integration/baseline/quirin-audit')
assert hashlib.sha256(base.read_bytes()).hexdigest()=='1e9e70c262d6e28a423cde0ad96eec3d0bb899fc599a07aaa9b3195de3cc06dd'
gate=json.loads((private/'gate-1/commands.json').read_text());assert len(gate['steps'])==9 and all(step['exit']==0 for step in gate['steps'])
tip=private/'gate-1/quirin-audit';tip_receipt=json.loads((tip.parent/'binary.json').read_text())
assert tip_receipt['source']=='ccb354a7d8279faf6078d611919ff5fe56ad7ed4'
assert tip_receipt['sha256']==hashlib.sha256(tip.read_bytes()).hexdigest()
inputs=private/'base-inputs';manifest=json.loads((private/'base-input-manifest.json').read_text())
for path,digest in manifest['files'].items():assert hashlib.sha256((inputs/path).read_bytes()).hexdigest()==digest,path
# Build the report-owned reference verifier against the exact frozen release
# libraries; it does not perform or claim C source reanalysis.
helper=private/'check-production-bindings';deps=repo/'target/release/deps'
command=['rustc','--edition=2024',str(repo/'docs/reports/libc-reading-site-evidence/scripts/check_production_bindings.rs'),'-L','dependency='+str(deps)]
libraries={}
for name in ['qcore_audit','qcore_c_gateway','qcore_contract','qcore_schemas','serde_json']:
    found=list(deps.glob('lib'+name+'-*.rlib'));assert len(found)==1,(name,found)
    libraries[name]={'path':str(found[0]),'sha256':hashlib.sha256(found[0].read_bytes()).hexdigest()};command+=['--extern',name+'='+str(found[0])]
command+=['-o',str(helper)]
with (private/'binding-helper-build-1.log').open('wb') as log:code=subprocess.run(command,cwd=repo,stdout=log,stderr=subprocess.STDOUT).returncode
(private/'binding-helper-build-1.json').write_text(json.dumps(dict(command=command,exit=code,source_sha256=hashlib.sha256((repo/'docs/reports/libc-reading-site-evidence/scripts/check_production_bindings.rs').read_bytes()).hexdigest(),libraries=libraries),indent=2)+'\n')
if code:raise SystemExit(code)
records=[]
for side,binary,commit in [('before',base,'c5c298db2ab7dedb5d39d29f4cfd097727e267b2'),('after',tip,tip_receipt['source'])]:
    running=[]
    for route in ['system','slice']:
        out=private/'runs-1'/route/side
        command=['python3',str(repo/'docs/reports/libc-reading-site-evidence/scripts/measure.py'),'--contract-root',str(inputs),'--jobs','8',route,str(binary),commit,str(out)]
        print(route,side,'start',flush=True);start=time.time();process=subprocess.Popen(command,cwd=repo);running.append((route,process,command,start))
    for route,process,command,start in running:
        code=process.wait();records.append(dict(route=route,side=side,command=command,exit=code,seconds=time.time()-start));print(route,side,code,flush=True)
        (private/'measurement-commands.json').write_text(json.dumps(records,indent=2)+'\n')
    if any(r['exit'] for r in records):raise SystemExit(1)
for side,binary,commit in [('before',base,'c5c298db2ab7dedb5d39d29f4cfd097727e267b2'),('after',tip,tip_receipt['source'])]:
    out=private/'runs-1/corpus'/side
    command=['python3',str(repo/'docs/reports/libc-reading-site-evidence/scripts/measure.py'),'--contract-root',str(inputs),'--jobs','16','corpus',str(binary),commit,str(out)]
    print('corpus',side,'start',flush=True);start=time.time();code=subprocess.run(command,cwd=repo).returncode
    records.append(dict(route='corpus',side=side,command=command,exit=code,seconds=time.time()-start));print('corpus',side,code,flush=True)
    (private/'measurement-commands.json').write_text(json.dumps(records,indent=2)+'\n')
    if code:raise SystemExit(code)
