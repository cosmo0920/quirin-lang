from pathlib import Path
import json,subprocess,os,datetime,hashlib,shutil,sys
root=Path.cwd();p=root/'private/w48-cmt-tracked-interior';base=json.loads((p/'config.json').read_text())['base'];assert shutil.disk_usage(root).free>=40*1024**3
subprocess.run(['git','diff','--exit-code',base,'--','crates','compiler','proof','scripts','contract','providers'],check=True)
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
for name,cmd in [('submodules',['git','submodule','update','--init','--recursive']),('build',['env','CARGO_BUILD_JOBS=6','QCORE_AUDIT_BUILD_COMMIT='+base,'cargo','build','--release','-p','qcore-audit','--bin','quirin-audit'])]:
 record={'command':cmd,'head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'semantic_base':base,'started':datetime.datetime.now(datetime.timezone.utc).isoformat()}
 with (p/('base-'+name+'.log')).open('wb') as f:code=subprocess.run(cmd,stdout=f,stderr=f).returncode
 record.update(exit=code,ended=datetime.datetime.now(datetime.timezone.utc).isoformat());(p/('base-'+name+'.json')).write_text(json.dumps(record,indent=2)+'\n');(p/('base-'+name+'.exit')).write_text(str(code)+'\n')
 if code:raise SystemExit(code)
runtime=p/'base-runtime';(runtime/'target/release').mkdir(parents=True,exist_ok=False);(runtime/'scripts').mkdir();binary=runtime/'target/release/quirin-audit';shutil.copyfile(root/'target/release/quirin-audit',binary);binary.chmod(0o755);shutil.copyfile(p/'inputs/scripts/run_pinned_audit.py',runtime/'scripts/run_pinned_audit.py');(p/'base-binary.json').write_text(json.dumps({'source':base,'build':json.loads((p/'base-build.json').read_text()),'sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'binary':str(binary)},indent=2)+'\n')
cmd=['sh','docs/reports/w48-bu-cmt-map-metric-get/scripts/original_paths.sh','python3',str(p/'measure.py'),'base'];code=subprocess.run(cmd).returncode;(p/'base-complete.exit').write_text(str(code)+'\n');raise SystemExit(code)
