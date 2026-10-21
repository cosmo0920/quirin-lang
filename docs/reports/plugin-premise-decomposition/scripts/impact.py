"""Enumerate actual property-specific dependencies; never mint premise evidence."""
import argparse,collections,gzip,hashlib,json,pathlib,runpy
root=pathlib.Path.cwd();report=root/'docs/reports/plugin-premise-decomposition';module=runpy.run_path(str(report/'scripts/graph_metadata.py'));atoms=set(module['ATOMS']);premise=module['PREMISE'];index=json.loads((report/'bindings/index.json').read_text())['occurrences'];held={(v['property_view'],v['declaration']):v for v in index};coords=['artifact','property','semantic_profile','scope','bounds','assumptions','source_identity','build_identity','evidence_version']
p=argparse.ArgumentParser();p.add_argument('route');p.add_argument('tip',type=pathlib.Path);p.add_argument('out',type=pathlib.Path);a=p.parse_args();diag=json.loads((a.tip/'resource-diagnostics.json').read_text());null=[json.loads(x) for x in (a.tip/(a.route+'-null.jsonl')).read_text().splitlines() if x];potential=[];unrelated=[]
def context(row):
 c=row['hypothesis_context'];assert set(c)==set(coords) and all(isinstance(c[k],str) and c[k] for k in coords);return c
def add(view,row,identity,declarations):
 record={'view':view,'row':identity,'context':context(row),'status':row.get('status',row.get('null',{}).get('status')),'row_sha256':hashlib.sha256(json.dumps(row,sort_keys=True,separators=(',',':')).encode()).hexdigest()}
 if not declarations:unrelated.append(record);return
 record['bindings']=[]
 for declaration in sorted(declarations):
  binding=held[view,declaration];raw=(report/binding['manifest_path']).read_bytes();assert hashlib.sha256(raw).hexdigest()==binding['manifest_sha256'];manifest=json.loads(raw);assert set(manifest['atoms'])==atoms
  record['bindings'].append({'declaration':declaration,'manifest_sha256':binding['manifest_sha256'],'declared_source_graph_sha256':binding['graph_sha256'],'formal_atoms':{name:{key:item[key] for key in ['atom_sha256','proposition','prerequisites','environment']} for name,item in manifest['atoms'].items()},'source_binding':'open'})
 potential.append(record)
for ui,unit in enumerate(diag['units']):
 for ri,row in enumerate(unit['functions']):
  declarations=[]
  for name,dependencies in row.get('contract_dependencies',{}).get('hypothesis_dependencies',{}).items():
   found=atoms & set(dependencies)
   if found:assert found==atoms and ('resource',name) in held;declarations.append(name)
  add('resource',row,{'unit_index':ui,'row_index':ri,'library':unit.get('library'),'file':unit.get('file'),'function':row['function']},declarations)
for ni,row in enumerate(null):
 references=row.get('hypothesis_binding_requirements',[]);declarations=[]
 for requirement in references:
  if requirement['premise']!=premise:continue
  binding=held['null',requirement['declaration']];assert requirement['manifest_sha256']==binding['manifest_sha256'];assert requirement['graph_sha256']==binding['graph_sha256'];declarations.append(requirement['declaration'])
 add('null',row,{'null_row_index':ni,'artifact':row.get('artifact'),'function':row.get('function')},declarations)
selected=potential[0] if potential else None;affected=[];other=[]
if selected:
 target=selected['bindings'][0]['declaration'];view=selected['view'];ctx=selected['context']
 for record in potential:
  if record['view']==view and any(b['declaration']==target for b in record['bindings']) and record['context']==ctx:affected.append(record['row'])
  else:other.append(record['row'])
result={'route':a.route,'source_binding':'open','actual_unchanged_production_dispositions':'all open; no automatic Unsafe','potential_dependent_rows':potential,'unrelated_rows':unrelated,'hypothetical_guarded_refutation_control':{'actual_source_error':False,'selected_graph_binding':None if not selected else selected['bindings'][0],'independently_held_context':None if not selected else selected['context'],'affected_exact_graph_property_context_rows':affected,'other_graph_property_or_context_rows_unchanged':other,'reference':'Rust exact Graph/Context refusal controls + conditional Lean callback_without_registration_or_default_is_refuted; not applied as production evidence'},'source_finding':{'path':'src/flb_plugin_proxy.c','sha256':'00ca65b30aabad04495496fdac049e8ef95ed9adef0c48eaefcf3e7f15e8e87f','detected':'proxy creator ignores registration return','affected_prerequisite':'fluent_go_proxy_registration_success cannot be inferred from proxy allocation or create return','prerequisite_disposition':'open','propagation':['loader/trace-selection source-to-model assignment remains unestablished','all selected environment-dependent proposed atom/outer-bridge instances remain Open in the listed exact property contexts'],'distinction':'This is Fluent proxy registration, not an RK interceptor setter; it refutes neither loadedRegistration nor either original envelope. Missing guards cannot become supported source evidence.','negative_reference':'outer_catalog_controls.py unsupported-source-truth-even-if-expected-changed'},'counts':{'potential_by_view':dict(collections.Counter(r['view'] for r in potential)),'unrelated_by_view':dict(collections.Counter(r['view'] for r in unrelated)),'exact_context_hypothetical_affected':len(affected),'other_context_or_graph_unchanged':len(other)},'production_verdict_changes':[],'legacy_premise_disposition_changes':[]}
a.out.write_bytes(gzip.compress((json.dumps(result,indent=2)+'\n').encode(),mtime=0));print(json.dumps(result['counts']))
