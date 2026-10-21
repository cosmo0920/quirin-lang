"""Exact complete compile identity preimages from source-pinned serde order."""
import argparse,copy,hashlib,json,pathlib,subprocess
p=argparse.ArgumentParser();p.add_argument('route');p.add_argument('out',type=pathlib.Path);a=p.parse_args();root=pathlib.Path.cwd();private=root/'private/plugin-premise-decomposition';base=private/'measure-1'/a.route/'base';tip=private/'measure-1'/a.route/'tip';read=lambda x:json.loads(x.read_bytes());dumps=lambda x:json.dumps(x,ensure_ascii=False,separators=(',',':'));sha=lambda s:hashlib.sha256(s.encode()).hexdigest();known={};records=[]
# Exact schema/field order is independently pinned to both actual producer sources.
files=['crates/qcore-audit/src/compile_db.rs','crates/qcore-c-gateway/src/clang.rs','crates/qcore-c-gateway/src/legacy_c.rs'];producer={}
for f in files:
 b=subprocess.check_output(['git','show','32e6f5dda334d6a017a1be4dcb02f72a9138c6a6:'+f]);t=subprocess.check_output(['git','show','c1625baf713e4d8498c39282238675749237005d:'+f]);assert b==t;producer[f]=hashlib.sha256(b).hexdigest()
fields='schema_version compiler declared_driver directory file arguments dialect function_scope source_input preprocessor_options include_paths definitions raw_source_sha256 preprocessed_source_sha256 preprocessing_error dependencies dialect_differences_sha256'.split()
def task_bytes(v):
 assert set(v)<=set(fields) and set(fields[:-1])<=set(v)
 x={k:copy.deepcopy(v[k]) for k in fields if k in v};c=x['compiler'];assert set(c)=={'program','version','target'};x['compiler']={k:c[k] for k in ['program','version','target']}
 s=x['source_input']
 if isinstance(s,dict):
  assert set(s)=={'DeclaredFile'} and set(s['DeclaredFile'])=={'spelled','directory'};x['source_input']={'DeclaredFile':{k:s['DeclaredFile'][k] for k in ['spelled','directory']}}
 else:assert s in ['AnonymousText','HistoricalStdin']
 x['dependencies']=dict(sorted(x['dependencies'].items()));return dumps(x)
left=read(base/'resource-diagnostics.json');right=read(tip/'resource-diagnostics.json');assert len(left['units'])==len(right['units'])
roots={s:read(private/'route-environments'/a.route/(s+'-environment-hashes.json'))['source_assumptions_sha256'] for s in ['base','tip']}
for bu,tu in zip(left['units'],right['units']):
 assert (bu['library'],bu['file'])==(tu['library'],tu['file']);bc=bu.get('compile_configuration');tc=tu.get('compile_configuration')
 if bc is None or tc is None:assert bc==tc;continue
 if 'compile_task_identity' not in bc or 'compile_task_identity' not in tc:assert bc==tc;continue
 tasks=[];cfgs=[]
 for side,u,cfg in [('base',bu,bc),('tip',tu,tc)]:
  task=cfg['compile_task_identity'];assert task['dependencies']['resource_contract_sources']==roots[side];digest=sha(task_bytes(task));contexts=[r['hypothesis_context'] for r in u['functions'] if 'hypothesis_context' in r];assert all(c['build_identity']==digest for c in contexts), (side,u['file'],'task canonical preimage')
  c=copy.deepcopy(cfg);published=c.pop('inventory_sha256');assert sha(dumps(c))==published,(side,u['file'],'full configuration preimage');cfgs.append((c,published));tasks.append((task,digest))
 restored=copy.deepcopy(tasks[1][0]);restored['dependencies']['resource_contract_sources']=roots['base'];assert restored==tasks[0][0],u['file']
 restored_cfg=copy.deepcopy(cfgs[1][0]);restored_cfg['dependencies']['resource_contract_sources']=roots['base'];restored_cfg['compile_task_identity']['dependencies']['resource_contract_sources']=roots['base'];assert restored_cfg==cfgs[0][0],u['file']
 negative=copy.deepcopy(tasks[0][0]);negative['dependencies']['resource_contract_sources']='0'*64;assert sha(task_bytes(negative))!=tasks[0][1]
 for before,after in [(tasks[0][1],tasks[1][1]),(cfgs[0][1],cfgs[1][1])]:
  if after in known:assert known[after]==before
  if before!=after:known[after]=before
 records.append({'library':bu['library'],'file':bu['file'],'base_task_sha256':tasks[0][1],'tip_task_sha256':tasks[1][1],'base_configuration_sha256':cfgs[0][1],'tip_configuration_sha256':cfgs[1][1],'all_context_build_identities_recomputed':True,'complete_compile_tasks_and_configurations_equal_after_single_proven_source_dependency_restoration':True,'wrong_source_dependency_negative':True})
a.out.write_text(json.dumps({'route':a.route,'records':records,'exact_strings':known,'producer_source_sha256':producer,'canonical_encoding':'unchanged production serde struct order; all complete observed context digests checked; receipt-bearing units additionally constructed by actual public compile_task_identity in Rust'},indent=2)+'\n');print(a.route,len(records))
