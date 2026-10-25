from pathlib import Path
import subprocess, json, datetime, os, time
p=Path('private/w48-cmt-tracked-interior');root_gate=json.load(open(p/'latest-gate.json'));assert root_gate['exit']==0, 'finish root gate repairs before provider measurement';out=p/'provider-api-gate';out.mkdir(exist_ok=False)
env=dict(os.environ,CARGO_BUILD_JOBS='6');head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
packages=p/'packages-api'
commands=[('contracts-check',['python3','providers/contracts.py','check']),('tests',['cargo','test','--manifest-path','providers/Cargo.toml','--workspace','--locked','--offline']),('fmt',['cargo','fmt','--manifest-path','providers/Cargo.toml','--all','--','--check']),('clippy',['cargo','clippy','--manifest-path','providers/Cargo.toml','--workspace','--all-targets','--locked','--offline','--','-D','warnings']),('python',['python3','-m','unittest','discover','-s','providers/tests']),('package-build',['python3','providers/package.py','build','--output',str(packages)])]+[(f'verify-{name}',['python3','providers/package.py','verify',str(packages/('qcore.'+name))]) for name in ['chunkio','cmetrics','ctraces','libevent','librdkafka']]
r={'head':head,'steps':[]}
for name,cmd in commands:
 assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
 assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==head
 start=time.monotonic();log=out/(name+'.log');print('START',name,flush=True)
 with log.open('wb') as f:code=subprocess.run(cmd,stdout=f,stderr=f,env=env).returncode
 (out/(name+'.exit')).write_text(str(code)+'\n');r['steps'].append({'name':name,'command':cmd,'exit':code,'seconds':time.monotonic()-start,'log':str(log)});(out/'gate.json').write_text(json.dumps(r,indent=2)+'\n');print('END',name,code,flush=True)
r['exit']=int(any(s['exit'] for s in r['steps']));(out/'gate.json').write_text(json.dumps(r,indent=2)+'\n');raise SystemExit(r['exit'])
