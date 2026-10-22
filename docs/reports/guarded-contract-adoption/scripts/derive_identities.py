"""Exact input and complete typed-producer identities; only independently checked guarded source recovery."""
import argparse,collections,copy,hashlib,json,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('route');p.add_argument('base',type=Path);p.add_argument('tip',type=Path);p.add_argument('base_environments',type=Path);p.add_argument('tip_environments',type=Path);p.add_argument('out',type=Path);p.add_argument('--reading-proof',type=Path,required=True);a=p.parse_args()
read=lambda p:json.loads(p.read_bytes());sha=lambda b:hashlib.sha256(b).hexdigest();js=lambda x:json.dumps(x,ensure_ascii=False,separators=(',',':'),sort_keys=True)
inv={s:read(d/'invocation.json') for s,d in [('base',a.base),('tip',a.tip)]};roots={s:Path(v['inputs_snapshot']) for s,v in inv.items()};envdirs={'base':a.base_environments/a.route,'tip':a.tip_environments/a.route};env={s:read(d/(s+'-environment.json')) for s,d in envdirs.items()};hashes={s:read(d/(s+'-environment-hashes.json')) for s,d in envdirs.items()};known={};reasons={};files=[]
def add(new,old,why,fields):
 if new==old:return
 assert isinstance(new,str) and isinstance(old,str)
 assert len(new)==len(old)==64 and all(c in '0123456789abcdef' for c in new+old),'only exact digests, no module/atom/text inverse'
 if new in known:assert known[new]==old
 known[new]=old;reasons.setdefault(sha(new.encode()),[]).append({'proof':why,'allowed_field_kinds':fields})
proof=read(a.reading_proof)
assert proof['source_binding']=='open' and proof['truth_transport'] is False
bridge=proof['guarded_metadata_input_bridge']
assert bridge['plan_sha256']=='f19cb3d1f20df83b5eb1396d1baf9dadeb9d6ca7dd3570edba10e42eb817219e'
for side in ['base','tip']:
 assert proof['input_sha256'][side+'_environment']==sha((envdirs[side]/(side+'-environment.json')).read_bytes())
approved_files={f['relative']:f for f in bridge['changed_runtime_files']}
def normalize(value,side,path=()):
 if isinstance(value,str) and side=='tip' and value==bridge['after_text']:
  assert len(path)==2 and path[0]=='source_assumptions' and str(path[1]).endswith(bridge['source_module'])
  return bridge['before_text']
 if isinstance(value,str):
  root=str(roots[side]);
  if root not in value:return value
  assert value.startswith(root+'/') and value.count(root)==1,('root in non-file scalar',path)
  assert path and path[-1] in {'module','source_module','lock','path','$key'},('unexpected file field',path)
  actual=Path(value).resolve();relative=actual.relative_to(roots[side].resolve());other=roots['tip' if side=='base' else 'base']/relative
  assert actual.is_file() and other.is_file(),('missing held metadata file',path)
  if actual.read_bytes()!=other.read_bytes():
   approved=approved_files.get(str(relative));assert approved,('nonidentical source or pin file outside sealed bridge',path)
   assert sha(actual.read_bytes())==approved[('before' if side=='base' else 'after')+'_sha256']
   assert sha(other.read_bytes())==approved[('after' if side=='base' else 'before')+'_sha256']
  files.append({'side':side,'path':path,'relative':str(relative),'sha256':sha(actual.read_bytes())})
  return '$INPUTS'+value[len(root):]
 if isinstance(value,list):return [normalize(v,side,path+(i,)) for i,v in enumerate(value)]
 if isinstance(value,dict):
  pairs=[(normalize(k,side,path+('$key',)),normalize(v,side,path+(k,))) for k,v in value.items()];assert len(dict(pairs))==len(pairs);return dict(pairs)
 return value
for s in env:
 raw=(envdirs[s]/(s+'-environment.json')).read_bytes();assert sha(raw)==hashes[s]['environment_sha256']
 assert sha(json.dumps(env[s]['source_assumptions'],ensure_ascii=False,separators=(',',':'),sort_keys=True).encode())==hashes[s]['source_assumptions_sha256']
assert normalize(env['base'],'base')==normalize(env['tip'],'tip'),'complete root environment mismatch; no source/proposition inverse permitted'
assert hashes['base']['function_table_sha256']==hashes['tip']['function_table_sha256']
for relative,record in approved_files.items():
 for side in ['base','tip']:
  assert sha((roots[side]/relative).read_bytes())==record[('before' if side=='base' else 'after')+'_sha256']
 add(record['after_sha256'],record['before_sha256'],'sealed guarded-v2 whole-source recovery or complete official pin/inventory reconstruction',['source_sha256','contract_source_sha256','module_sha256','contract_text_sha256'])
before_lock=read(roots['base']/'providers/qcore-provider-librdkafka/contracts.lock')
after_lock=read(roots['tip']/'providers/qcore-provider-librdkafka/contracts.lock')
bd=next(e for e in before_lock['pinned'] if e['function']=='cJSON_DetachItemViaPointer')['contract_sha256']
td=next(e for e in after_lock['pinned'] if e['function']=='cJSON_DetachItemViaPointer')['contract_sha256']
add(td,bd,'complete official lock reconstruction, unchanged reviewed sites, independently checked guarded source clause digest',['contract_sha256','contract_text_sha256'])

for key,fields in [('environment_sha256',['environment_sha256','home_environment_sha256']),('source_assumptions_sha256',['resource_contract_sources','source_assumptions_sha256','assumptions'])]:add(hashes['tip'][key],hashes['base'][key],'complete independently exported typed environment and byte-exact paired source/pin files',fields)
# Hold complete actual producer sources, not an order inferred from output rows.
producer={};base_commit=inv['base']['analyzer_source'];tip_commit=inv['tip']['analyzer_source']
for f in ['crates/qcore-audit/src/compile_db.rs','crates/qcore-c-gateway/src/clang.rs','crates/qcore-c-gateway/src/legacy_c.rs','crates/qcore-audit/src/report.rs']:
 b=subprocess.check_output(['git','show',base_commit+':'+f]);t=subprocess.check_output(['git','show',tip_commit+':'+f]);assert b==t,('producer source changed; requires new explicit proof',f);producer[f]=sha(b)
fields='schema_version compiler declared_driver directory file arguments dialect function_scope source_input preprocessor_options include_paths definitions raw_source_sha256 preprocessed_source_sha256 preprocessing_error dependencies dialect_differences_sha256'.split()
def task_bytes(v):
 assert set(v)<=set(fields) and set(fields[:-1])<=set(v)
 x={k:copy.deepcopy(v[k]) for k in fields if k in v};c=x['compiler'];assert set(c)=={'program','version','target'};x['compiler']={k:c[k] for k in ['program','version','target']}
 s=x['source_input']
 if isinstance(s,dict):assert set(s)=={'DeclaredFile'} and set(s['DeclaredFile'])=={'spelled','directory'};x['source_input']={'DeclaredFile':{k:s['DeclaredFile'][k] for k in ['spelled','directory']}}
 else:assert s in ['AnonymousText','HistoricalStdin']
 x['dependencies']=dict(sorted(x['dependencies'].items()));return json.dumps(x,ensure_ascii=False,separators=(',',':')).encode()
diagnostics={s:read(d/'resource-diagnostics.json') for s,d in [('base',a.base),('tip',a.tip)]};cfgrecords=[]
assert len(diagnostics['base']['units'])==len(diagnostics['tip']['units'])
contexts={s:collections.defaultdict(collections.deque) for s in inv}
for bu,tu in zip(diagnostics['base']['units'],diagnostics['tip']['units']):
 assert (bu['library'],bu['file'])==(tu['library'],tu['file'])
 for s,u in [('base',bu),('tip',tu)]:
  for r in u['functions']:
   if 'hypothesis_context'in r:contexts[s][(u['library'],u['file'],r['function'])].append(r['hypothesis_context'])
 bc,tc=bu.get('compile_configuration'),tu.get('compile_configuration')
 if bc is None or tc is None:assert bc==tc;continue
 if 'compile_task_identity' not in bc or 'compile_task_identity' not in tc:assert bc==tc;continue
 tasks=[];cfgs=[]
 for s,u,cfg in [('base',bu,bc),('tip',tu,tc)]:
  task=cfg['compile_task_identity'];assert task['dependencies']['resource_contract_sources']==hashes[s]['source_assumptions_sha256'];digest=sha(task_bytes(task));assert all(r['hypothesis_context']['build_identity']==digest for r in u['functions'] if 'hypothesis_context'in r)
  c=copy.deepcopy(cfg);published=c.pop('inventory_sha256');assert sha(js(c).encode())==published;tasks.append((task,digest));cfgs.append((c,published))
 restored=copy.deepcopy(tasks[1][0]);restored['dependencies']['resource_contract_sources']=hashes['base']['source_assumptions_sha256'];assert restored==tasks[0][0]
 restored_cfg=copy.deepcopy(cfgs[1][0]);restored_cfg['dependencies']['resource_contract_sources']=hashes['base']['source_assumptions_sha256'];restored_cfg['compile_task_identity']['dependencies']['resource_contract_sources']=hashes['base']['source_assumptions_sha256'];assert restored_cfg==cfgs[0][0]
 negative=copy.deepcopy(tasks[0][0]);negative['dependencies']['resource_contract_sources']='0'*64;assert sha(task_bytes(negative))!=tasks[0][1]
 add(tasks[1][1],tasks[0][1],'full compile task typed serde canonical preimage and held producer equality',['build_identity']);add(cfgs[1][1],cfgs[0][1],'full compile configuration canonical preimage',['inventory_sha256']);cfgrecords.append({'library':bu['library'],'file':bu['file'],'base_task':tasks[0][1],'tip_task':tasks[1][1],'base_inventory':cfgs[0][1],'tip_inventory':cfgs[1][1],'wrong_dependency_negative':True})
nulls={s:[json.loads(l) for l in (d/(a.route+'-null.jsonl')).read_bytes().splitlines() if l] for s,d in [('base',a.base),('tip',a.tip)]};assert len(nulls['base'])==len(nulls['tip']);nullrecords=[]
for b,t in zip(nulls['base'],nulls['tip']):
 key=tuple(b[k] for k in ['library','file','function']);assert key==tuple(t[k] for k in ['library','file','function']);assert b['identity']==t['identity'],'null semantic/source identity changed'
 assert b['identity']['resource_contracts_sha256']==hashes['base']['function_table_sha256']
 for s,r in [('base',b),('tip',t)]:
  if 'hypothesis_context'not in r:continue
  resource=contexts[s][key].popleft();derived=sha(js({'custody':resource['assumptions'],'null_identity':r['identity']}).encode());assert derived==r['hypothesis_context']['assumptions'];assert js({'scope':r['null']['scope'],'assumptions':r['null']['assumptions']})==r['hypothesis_context']['scope'];assert r['hypothesis_context']['property']=='c.null.deref.v1'
 assert ('hypothesis_context'in b)==('hypothesis_context'in t)
 if 'hypothesis_context'in b:
  add(t['hypothesis_context']['assumptions'],b['hypothesis_context']['assumptions'],'complete actual null identity plus paired actual custody assumption canonical preimages',['assumptions']);nullrecords.append({'artifact':key,'base':b['hypothesis_context']['assumptions'],'tip':t['hypothesis_context']['assumptions']})
result={'status':'complete independently held input/environment and producer preimages','exact_strings':known,'justifications_by_new_string_sha256':reasons,'input_roots':{s:str(r) for s,r in roots.items()},'environment_hashes':hashes,'paired_file_records':files,'producer_source_sha256':producer,'compile_configurations':cfgrecords,'null_composite_contexts':nullrecords,'source_binding':'open','no_unchecked_source_text_or_atom_inverse':True,'all_fields_retained':True,'guarded_source_bridge':bridge,'reading_proof_sha256':sha(a.reading_proof.read_bytes())};a.out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'route':a.route,'substitutions':len(known),'configurations':len(cfgrecords),'null_contexts':len(nullrecords)}))
