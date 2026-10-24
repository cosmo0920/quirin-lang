"""TIP campaign; run below this card's held validation shell in original paths."""
from pathlib import Path
import datetime,hashlib,json,os,shutil,subprocess,sys
root=Path.cwd();p=root/'private/w48-cmt-map';snapshot=p/'base-inputs-v2';runtime=p/'tip-runtime'
sha=lambda path:hashlib.sha256(Path(path).read_bytes()).hexdigest()
freeze=json.loads((p/'semantic-freeze.json').read_text())['head'];gate=json.loads((p/'latest-gate.json').read_text());assert gate['exit']==0 and gate['head']==freeze
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==freeze
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
slot=json.loads((p/'development-lock.json').read_text());ancestor=os.getppid()
while ancestor>1 and ancestor!=slot['pid']:ancestor=int(Path(f'/proc/{ancestor}/stat').read_text().rsplit(')',1)[1].split()[1])
assert ancestor==slot['pid']
build=json.loads((p/'steps/build-tip-1.json').read_text());assert build['exit']==0 and build['head']==freeze
assert 'QCORE_AUDIT_BUILD_COMMIT='+freeze in build['command']
assert not runtime.exists();(runtime/'target/release').mkdir(parents=True);(runtime/'scripts').mkdir()
binary=runtime/'target/release/quirin-audit';shutil.copyfile(root/'target/release/quirin-audit',binary);binary.chmod(0o755)
runner=runtime/'scripts/run_pinned_audit.py';runner.write_bytes((snapshot/'scripts/run_pinned_audit.py').read_bytes());assert sha(runner)==sha(snapshot/'scripts/run_pinned_audit.py')
(p/'tip-build-receipt.json').write_text(json.dumps({'semantic_source':freeze,'binary':str(binary),'sha256':sha(binary),'build':build,'runner_sha256':sha(runner),'profiles_snapshot':str(snapshot)},indent=2)+'\n')
primary=Path('/home/cosmo/Documents/GitHub/qcore-c');inputs=primary/'build/cycle19';source=inputs/'c-gateway-wave-pin/fluent-bit-clang-source';database=source/'build/compile_commands.json';monkey=inputs/'c-gateway-wave-pin/monkey/monkey-build/compile_commands.json';package=inputs/'provider-packages-wave20/qcore.chunkio';link=root/'docs/reports/detach-result-wrapper-composition/link-map.corpus.json';manifest=json.loads((snapshot/'manifest.json').read_text());out=p/'tip-corpus'
def identities():
 return {'binary':sha(binary),'manifest':sha(snapshot/'manifest.json'),'inputs':{name:sha(snapshot/name) for name in manifest},'source':{'head':subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip(),'tracked_delta_sha256':hashlib.sha256(subprocess.check_output(['git','-C',str(source),'diff','--no-ext-diff','--binary','HEAD'])).hexdigest()},'database':sha(database),'monkey_database':sha(monkey),'namespace_wrapper':sha(root/'docs/reports/w48-bu-cmt-map-metric-get/scripts/original_paths.sh'),'link_map':sha(link),'driver':sha(__file__),'runner':sha(runner),'package_files':{str(f.relative_to(package)):sha(f) for f in sorted(package.rglob('*')) if f.is_file()}}
assert shutil.disk_usage(root).free>=40*1024**3
before=identities();assert before['inputs']==manifest
env=dict(os.environ,QCORE_PROFILES=str(snapshot/'contract/profiles'),QCORE_CROSS_UNIT_LINK_MAP=str(link),QCORE_NULL_PROPERTY=str(p/'tip-corpus-null.jsonl'),QCORE_OUTER_PROJECTION=str(p/'tip-corpus-outer.json'))
cmd=['python3',str(runner),str(out),freeze,'--inputs-root',str(inputs),'--jobs','4']
r={'semantic_source':freeze,'working_head_before':freeze,'command':cmd,'cwd':str(root),'snapshot':str(snapshot),'environment':{k:env[k] for k in ['QCORE_PROFILES','QCORE_CROSS_UNIT_LINK_MAP','QCORE_NULL_PROPERTY','QCORE_OUTER_PROJECTION']},'clang':subprocess.check_output(['/usr/bin/clang','--version'],text=True).splitlines()[0],'started':datetime.datetime.now(datetime.timezone.utc).isoformat(),'before':before}
(p/'tip-corpus-invocation.json').write_text(json.dumps(r,indent=2)+'\n')
with (p/'tip-corpus-driver.log').open('wb') as log:code=subprocess.run(cmd,env=env,stdout=log,stderr=log).returncode
r.update(audit_exit=code,after=identities(),ended=datetime.datetime.now(datetime.timezone.utc).isoformat(),working_head_after=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),free_bytes_after=shutil.disk_usage(root).free);r['stable']=r['before']==r['after']
if not r['stable'] or r['free_bytes_after']<40*1024**3:code=101
r['exit']=code;(p/'tip-corpus-invocation.json').write_text(json.dumps(r,indent=2)+'\n');(p/'tip-corpus.exit').write_text(str(code)+'\n')
if code:raise SystemExit(code)
commands=[]
for route in ('system','slice'):
 cmd=['python3','docs/reports/w48-bu-cmt-map-metric-get/scripts/measure_configured.py',route,str(p/('tip-'+route)),'--binary',str(binary),'--snapshot',str(snapshot),'--semantic-source',freeze,'--link-map',str(link),'--primary-root',str(primary)]
 with (p/('tip-'+route+'-driver.log')).open('wb') as log:code=subprocess.run(cmd,stdout=log,stderr=log).returncode
 (p/('tip-'+route+'.exit')).write_text(str(code)+'\n');commands.append({'route':route,'command':cmd,'exit':code});(p/'tip-configured-jobs.json').write_text(json.dumps(commands,indent=2)+'\n')
 if code:raise SystemExit(code)
cmd=['python3','docs/reports/w48-bottom-up-callee-bodies/scripts/bottom_up_plan.py',str(out/'resource-diagnostics.json'),str(p/'tip-plan.json'),'--top','20']
with (p/'tip-plan.log').open('wb') as log:code=subprocess.run(cmd,stdout=log,stderr=log).returncode
(p/'tip-plan.exit').write_text(str(code)+'\n');(p/'tip-campaign.exit').write_text(str(code)+'\n');raise SystemExit(code)
