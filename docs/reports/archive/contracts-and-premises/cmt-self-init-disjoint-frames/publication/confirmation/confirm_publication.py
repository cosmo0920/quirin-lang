from pathlib import Path
import subprocess,json,datetime,os,hashlib,re,sys
p=Path('private/cmt-disjoint-frames/publication-confirmation-1');p.mkdir(exist_ok=False)
cmd=['python3','-m','unittest','discover','-s','scripts/tests'];env=dict(os.environ,QCORE_CLANG='/usr/lib/llvm-21/bin/clang',CARGO_BUILD_JOBS='6')
r={'command':cmd,'cwd':str(Path.cwd()),'working_head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'environment_overrides':{k:env[k] for k in ['QCORE_CLANG','CARGO_BUILD_JOBS']},'started':datetime.datetime.now(datetime.timezone.utc).isoformat(),'log':str(p/'python.log')}
with (p/'python.log').open('wb') as f:code=subprocess.run(cmd,env=env,stdout=f,stderr=f).returncode
raw=(p/'python.log').read_bytes();matches=re.findall(rb'Ran (\d+) tests? in',raw)
r.update(exit=code,ended=datetime.datetime.now(datetime.timezone.utc).isoformat(),log_sha256=hashlib.sha256(raw).hexdigest(),tests=int(matches[-1]) if matches else None)
(p/'receipt.json').write_text(json.dumps(r,indent=2)+'\n');(p/'exit').write_text(str(code)+'\n');print(json.dumps(r,indent=2));sys.exit(code)
