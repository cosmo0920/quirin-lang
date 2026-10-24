from pathlib import Path
import datetime,hashlib,json,os,shutil,subprocess,sys,time
root=Path.cwd();p=root/'private/w48-cmt-map';snap=p/'base-inputs-v2';out=p/'base-corpus-v3'
inputs=Path('/home/cosmo/Documents/GitHub/qcore-c/build/cycle19');source=inputs/'c-gateway-wave-pin/fluent-bit-clang-source';database=inputs/'c-gateway-wave-pin/monkey/monkey-build/compile_commands.json';package=inputs/'provider-packages-wave20/qcore.chunkio';link=root/'docs/reports/detach-result-wrapper-composition/link-map.corpus.json';binary=snap/'target/release/quirin-audit'
sha=lambda path:hashlib.sha256(Path(path).read_bytes()).hexdigest()
manifest=json.loads((snap/'manifest.json').read_text())
def identities():
 return {'binary':sha(binary),'manifest':sha(snap/'manifest.json'),'inputs':{name:sha(snap/name) for name in manifest},'source':{'head':subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip(),'tracked_delta_sha256':hashlib.sha256(subprocess.check_output(['git','-C',str(source),'diff','--no-ext-diff','--binary','HEAD'])).hexdigest()},'database':sha(source/'build/compile_commands.json'),'monkey_database':sha(database),'namespace_wrapper':sha(root/'docs/reports/w48-bu-cmt-map-metric-get/scripts/original_paths.sh'),'link_map':sha(link),'driver':sha(__file__),'package_files':{str(f.relative_to(package)):sha(f) for f in sorted(package.rglob('*')) if f.is_file()}}
assert shutil.disk_usage(root).free>=40*1024**3
before=identities();assert before['inputs']==manifest
assert before['binary']=='b533088b903f10ad03f78ad4245267fe8b108e853fd410c744a657f32446adde'
env=dict(os.environ,QCORE_PROFILES=str(snap/'contract/profiles'),QCORE_CROSS_UNIT_LINK_MAP=str(link),QCORE_NULL_PROPERTY=str(p/'base-corpus-v3-null.jsonl'),QCORE_OUTER_PROJECTION=str(p/'base-corpus-v3-outer.json'))
cmd=['python3',str(snap/'scripts/run_pinned_audit.py'),str(out),'8991b7b58fcbd33da082bbb44e819d47a0fb2561','--inputs-root',str(inputs),'--jobs','4']
record={'semantic_source':'8991b7b58fcbd33da082bbb44e819d47a0fb2561','working_head_before':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'command':cmd,'cwd':str(root),'snapshot':str(snap),'environment':{k:env[k] for k in ['QCORE_PROFILES','QCORE_CROSS_UNIT_LINK_MAP','QCORE_NULL_PROPERTY','QCORE_OUTER_PROJECTION']},'clang':subprocess.check_output(['/usr/bin/clang','--version'],text=True).splitlines()[0],'started':datetime.datetime.now(datetime.timezone.utc).isoformat(),'before':before}
(p/'base-corpus-v3-invocation.json').write_text(json.dumps(record,indent=2)+'\n')
with (p/'base-corpus-v3-driver.log').open('wb') as log:code=subprocess.run(cmd,env=env,stdout=log,stderr=log).returncode
record.update(audit_exit=code,after=identities(),ended=datetime.datetime.now(datetime.timezone.utc).isoformat(),working_head_after=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),free_bytes_after=shutil.disk_usage(root).free)
record['stable']=record['before']==record['after']
if not record['stable'] or record['free_bytes_after']<40*1024**3:code=101
record['exit']=code;(p/'base-corpus-v3-invocation.json').write_text(json.dumps(record,indent=2)+'\n');(p/'base-corpus-v3.exit').write_text(str(code)+'\n')
if code:raise SystemExit(code)
planner=['python3','docs/reports/w48-bottom-up-callee-bodies/scripts/bottom_up_plan.py',str(out/'resource-diagnostics.json'),str(p/'base-plan-v3.json'),'--top','20']
with (p/'base-plan-v3.log').open('wb') as log:code=subprocess.run(planner,stdout=log,stderr=log).returncode
(p/'base-plan-v3.exit').write_text(str(code)+'\n')
if code==0:
 plan=json.loads((p/'base-plan-v3.json').read_text());names={'cmt_map_metric_get','map_metric_get_unlocked','metric_prepare_storage','metric_hash_lookup','map_metric_create','metric_index_add'};receipt={k:v for k,v in plan.items() if k!='rows'};receipt['rows']=[r for r in plan['rows'] if r['body'].rsplit('::',1)[-1] in names];(p/'base-plan-v3-cmetrics.json').write_text(json.dumps(receipt,indent=2)+'\n')
raise SystemExit(code)
