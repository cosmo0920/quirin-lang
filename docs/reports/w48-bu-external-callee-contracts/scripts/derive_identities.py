"""Independently replay actual typed identity preimages; retain all raw outputs."""
from pathlib import Path
import json,hashlib,copy,subprocess,collections
import argparse
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--evidence-root',type=Path,default=Path(__file__).parent);parser.add_argument('--repo',type=Path,default=Path.cwd());parser.add_argument('--routes',nargs='+',choices=['corpus','system','slice'],default=['corpus','system','slice']);args=parser.parse_args()
P=args.evidence_root.resolve();ROOT=args.repo.resolve()
read=lambda f:json.loads(f.read_bytes())
sha=lambda b:hashlib.sha256(b).hexdigest()
canon=lambda v:json.dumps(v,ensure_ascii=False,separators=(',',':'),sort_keys=True).encode()
fields='schema_version compiler declared_driver directory file arguments dialect function_scope source_input preprocessor_options include_paths definitions raw_source_sha256 preprocessed_source_sha256 preprocessing_error dependencies dialect_differences_sha256'.split()
def task_bytes(v):
 assert set(v)<=set(fields) and set(fields[:-1])<=set(v)
 x={k:copy.deepcopy(v[k]) for k in fields if k in v};c=x['compiler'];assert set(c)=={'program','version','target'};x['compiler']={k:c[k] for k in ['program','version','target']}
 s=x['source_input']
 if isinstance(s,dict):
  assert set(s)=={'DeclaredFile'} and set(s['DeclaredFile'])=={'spelled','directory'}
  x['source_input']={'DeclaredFile':{k:s['DeclaredFile'][k] for k in ['spelled','directory']}}
 else:assert s in ['AnonymousText','HistoricalStdin']
 x['dependencies']=dict(sorted(x['dependencies'].items()))
 return json.dumps(x,ensure_ascii=False,separators=(',',':')).encode()
proof=read(P/'environment-attribution.json');assert len(proof['records'])==3
assert all((P/'tip-1'/r/(r+'.exit')).exists() and (P/'tip-1'/r/(r+'.exit')).read_text().strip()=='0' for r in args.routes),'requested TIP native routes must complete successfully before identity attribution'
for route in args.routes:
 hashes={s:read(P/(s+'-environments')/route/(s+'-environment-hashes.json')) for s in ['base','tip']}
 inv={s:read(P/(s+'-3' if s=='base' else 'tip-1')/route/'invocation.json') for s in ['base','tip']}
 commits={s:inv[s]['analyzer_source'] for s in inv}; producers={}
 for f in ['crates/qcore-audit/src/compile_db.rs','crates/qcore-audit/src/projection.rs','crates/qcore-c-gateway/src/clang.rs','crates/qcore-c-gateway/src/legacy_c.rs','crates/qcore-c-gateway/src/null_deref.rs','crates/qcore-audit/src/report.rs']:
  b,t=[subprocess.check_output(['git','show',commits[s]+':'+f],cwd=ROOT) for s in ['base','tip']];assert b==t,(f,'producer changed');producers[f]=sha(b)
 maps=[]
 def add(b,t,fields,reason):
  if b!=t:maps.append({'base':b,'tip':t,'allowed_fields':fields,'reason':reason})
 for name,allowed in [('environment_sha256',['environment_sha256','home_environment_sha256']),('source_assumptions_sha256',['assumptions','resource_contract_sources','source_assumptions_sha256']),('function_table_sha256',['resource_contracts_sha256'])]:
  add(hashes['base'][name],hashes['tip'][name],allowed,'Complete actual Rust-exported environment byte preimage, reconstructed with exactly the reviewed additions.')
 add('contract-hypothesis-graph.v1:'+commits['base'],'contract-hypothesis-graph.v1:'+commits['tip'],['evidence_version'],'Actual independently held analyzer build commits; same producer source bytes.')
 docs={s:read(P/('base-3' if s=='base' else 'tip-1')/route/('corpus/resource-diagnostics.json' if route=='corpus' else 'resource-diagnostics.json')) for s in inv}
 contexts={s:collections.defaultdict(collections.deque) for s in inv};cfgrecords=[];legacy=[]
 assert len(docs['base']['units'])==len(docs['tip']['units'])
 for i,(bu,tu) in enumerate(zip(docs['base']['units'],docs['tip']['units'])):
  assert (bu['library'],bu['file'])==(tu['library'],tu['file'])
  for s,u in [('base',bu),('tip',tu)]:
   for r in u['functions']:
    if 'hypothesis_context'in r:contexts[s][(u['library'],u['file'],r['function'])].append(r['hypothesis_context'])
  bc,tc=bu.get('compile_configuration'),tu.get('compile_configuration');assert (bc is None)==(tc is None)
  if bc is None:
   assert route=='corpus';br={r['function']:r.get('hypothesis_context') for r in bu['functions']};tr={r['function']:r.get('hypothesis_context') for r in tu['functions']};assert br.keys()==tr.keys()
   for n in br:
    if br[n] is not None:assert br[n]['build_identity']==tr[n]['build_identity']
   legacy.append({'unit':i,'file':bu['file'],'all_build_identities_unchanged':True});continue
  if 'compile_task_identity'not in bc or 'compile_task_identity'not in tc:assert bc==tc;continue
  tasks=[];cfgs=[]
  for s,u,cfg in [('base',bu,bc),('tip',tu,tc)]:
   task=cfg['compile_task_identity'];assert task['dependencies']['resource_contract_sources']==hashes[s]['source_assumptions_sha256'];digest=sha(task_bytes(task))
   assert all(r['hypothesis_context']['build_identity']==digest for r in u['functions'] if 'hypothesis_context'in r)
   c=copy.deepcopy(cfg);published=c.pop('inventory_sha256');assert sha(canon(c))==published;tasks.append((task,digest));cfgs.append((c,published))
  restored=copy.deepcopy(tasks[1][0]);restored['dependencies']['resource_contract_sources']=hashes['base']['source_assumptions_sha256'];assert restored==tasks[0][0]
  restored=copy.deepcopy(cfgs[1][0]);restored['dependencies']['resource_contract_sources']=hashes['base']['source_assumptions_sha256'];restored['compile_task_identity']['dependencies']['resource_contract_sources']=hashes['base']['source_assumptions_sha256'];assert restored==cfgs[0][0]
  negative=copy.deepcopy(tasks[0][0]);negative['dependencies']['resource_contract_sources']='0'*64;assert sha(task_bytes(negative))!=tasks[0][1]
  add(tasks[0][1],tasks[1][1],['build_identity'],'Complete CompileTaskIdentity serde preimage; sole input change is reviewed resource contract source digest.')
  add(cfgs[0][1],cfgs[1][1],['inventory_sha256'],'Complete canonical compile configuration preimage; exact nested resource source dependency changes.')
  cfgrecords.append({'unit':i,'file':bu['file'],'base_task':tasks[0][1],'tip_task':tasks[1][1],'base_inventory':cfgs[0][1],'tip_inventory':cfgs[1][1],'negative_sensitivity':True})
 nulls={s:[json.loads(l) for l in (P/('base-3'if s=='base'else'tip-1')/route/(route+'-null.jsonl')).read_bytes().splitlines() if l] for s in inv};assert len(nulls['base'])==len(nulls['tip']);nullrecords=[]
 for i,(b,t) in enumerate(zip(nulls['base'],nulls['tip'])):
  key=tuple(b[k] for k in ['library','file','function']);assert key==tuple(t[k] for k in ['library','file','function'])
  for s,r in [('base',b),('tip',t)]:assert r['identity']['resource_contracts_sha256']==hashes[s]['function_table_sha256']
  restored=copy.deepcopy(t['identity']);restored['resource_contracts_sha256']=b['identity']['resource_contracts_sha256'];assert restored==b['identity'],'bounds/model/source changed beyond function table'
  assert ('hypothesis_context'in b)==('hypothesis_context'in t)
  for s,r in [('base',b),('tip',t)]:
   if 'hypothesis_context'not in r:continue
   resource=contexts[s][key].popleft();derived=sha(canon({'custody':resource['assumptions'],'null_identity':r['identity']}));assert derived==r['hypothesis_context']['assumptions']
   assert canon({'scope':r['null']['scope'],'assumptions':r['null']['assumptions']}).decode()==r['hypothesis_context']['scope'];assert r['hypothesis_context']['property']=='c.null.deref.v1'
  if 'hypothesis_context'in b:
   add(b['hypothesis_context']['assumptions'],t['hypothesis_context']['assumptions'],['assumptions'],'Actual complete null identity and actual custody assumption canonical preimage.')
   nullrecords.append({'row':i,'artifact':key,'base':b['hypothesis_context']['assumptions'],'tip':t['hypothesis_context']['assumptions']})
 out={'route':route,'exit':0,'environment_proof_sha256':sha((P/'environment-attribution.json').read_bytes()),'producer_sources':producers,'commits':commits,'exact_field_scoped_pairs':maps,'compile_configurations':cfgrecords,'legacy_build_identities':legacy,'null_composite_preimages':nullrecords,'fields_deleted':0,'fields_masked':0,'scope':'Actual identity attribution, not libc implementation, C extraction or source truth.'}
 (P/(route+'-identity-attribution.json')).write_text(json.dumps(out,indent=2)+'\n');print({'route':route,'configurations':len(cfgrecords),'legacy_units':len(legacy),'null_contexts':len(nullrecords)})
