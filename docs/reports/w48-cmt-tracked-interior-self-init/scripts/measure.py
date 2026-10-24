"""Fresh paired routes using one frozen input snapshot and explicit source IDs."""
from pathlib import Path
import json,hashlib,datetime,subprocess,os,sys,shutil
root=Path.cwd();p=root/'private/w48-cmt-tracked-interior';side=sys.argv[1];assert side in ['base','tip'];cfg=json.loads((p/'config.json').read_text());source_id=cfg[side] if side=='base' else json.loads((p/'semantic-freeze.json').read_text())['head'];snap=p/'inputs';runtime=p/(side+'-runtime');binary=runtime/'target/release/quirin-audit';runner=runtime/'scripts/run_pinned_audit.py';out=p/(side+'-corpus');assert not out.exists()
sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest();manifest=json.loads((snap/'manifest.json').read_text());primary=Path('/home/cosmo/Documents/GitHub/qcore-c');inputs=primary/'build/cycle19';csource=inputs/'c-gateway-wave-pin/fluent-bit-clang-source';database=csource/'build/compile_commands.json';monkey=inputs/'c-gateway-wave-pin/monkey/monkey-build/compile_commands.json';package=inputs/'provider-packages-wave20/qcore.chunkio';link=root/'docs/reports/detach-result-wrapper-composition/link-map.corpus.json';wrapper=root/'docs/reports/w48-bu-cmt-map-metric-get/scripts/original_paths.sh'
def identities():
 return {'binary':sha(binary),'runner':sha(runner),'manifest':sha(snap/'manifest.json'),'inputs':{n:sha(snap/n) for n in manifest},'source':{'head':subprocess.check_output(['git','-C',str(csource),'rev-parse','HEAD'],text=True).strip(),'tracked_delta_sha256':hashlib.sha256(subprocess.check_output(['git','-C',str(csource),'diff','--no-ext-diff','--binary','HEAD'])).hexdigest()},'database':sha(database),'monkey_database':sha(monkey),'link_map':sha(link),'namespace_wrapper':sha(wrapper),'driver':sha(__file__),'package_files':{str(f.relative_to(package)):sha(f) for f in sorted(package.rglob('*')) if f.is_file()}}
assert shutil.disk_usage(root).free>=40*1024**3
before=identities();assert before['inputs']==manifest and before['runner']==manifest['scripts/run_pinned_audit.py']
env=dict(os.environ,QCORE_PROFILES=str(snap/'contract/profiles'),QCORE_CROSS_UNIT_LINK_MAP=str(link),QCORE_NULL_PROPERTY=str(p/(side+'-corpus-null.jsonl')),QCORE_OUTER_PROJECTION=str(p/(side+'-corpus-outer.json')))
cmd=['python3',str(runner),str(out),source_id,'--inputs-root',str(inputs),'--jobs','4'];record={'semantic_source':source_id,'working_head_before':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'command':cmd,'cwd':str(root),'snapshot':str(snap),'environment':{k:env[k] for k in ['QCORE_PROFILES','QCORE_CROSS_UNIT_LINK_MAP','QCORE_NULL_PROPERTY','QCORE_OUTER_PROJECTION']},'clang':subprocess.check_output(['/usr/bin/clang','--version'],text=True).splitlines()[0],'started':datetime.datetime.now(datetime.timezone.utc).isoformat(),'before':before}
(p/(side+'-corpus-invocation.json')).write_text(json.dumps(record,indent=2)+'\n')
with (p/(side+'-corpus-driver.log')).open('wb') as f:code=subprocess.run(cmd,env=env,stdout=f,stderr=f).returncode
record.update(audit_exit=code,after=identities(),ended=datetime.datetime.now(datetime.timezone.utc).isoformat(),working_head_after=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),free_bytes_after=shutil.disk_usage(root).free);record['stable']=record['before']==record['after'];code=code if record['stable'] and record['free_bytes_after']>=40*1024**3 else 101;record['exit']=code;(p/(side+'-corpus-invocation.json')).write_text(json.dumps(record,indent=2)+'\n');(p/(side+'-corpus.exit')).write_text(str(code)+'\n')
if code:raise SystemExit(code)
cmd=['python3','docs/reports/w48-bottom-up-callee-bodies/scripts/bottom_up_plan.py',str(out/'resource-diagnostics.json'),str(p/(side+'-plan.json')),'--top','20']
with (p/(side+'-plan.log')).open('wb') as f:code=subprocess.run(cmd,stdout=f,stderr=f).returncode
(p/(side+'-plan.exit')).write_text(str(code)+'\n')
if code:raise SystemExit(code)
for route in ['system','slice']:
 cmd=['python3','docs/reports/w48-bu-cmt-map-metric-get/scripts/measure_configured.py',route,str(p/(side+'-'+route)),'--binary',str(binary),'--snapshot',str(snap),'--semantic-source',source_id,'--link-map',str(link),'--primary-root',str(primary)]
 with (p/(side+'-'+route+'-driver.log')).open('wb') as f:code=subprocess.run(cmd,stdout=f,stderr=f).returncode
 (p/(side+'-'+route+'.exit')).write_text(str(code)+'\n')
 if code:raise SystemExit(code)
(p/(side+'-campaign.exit')).write_text('0\n')
