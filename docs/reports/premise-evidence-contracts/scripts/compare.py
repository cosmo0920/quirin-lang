#!/usr/bin/env python3
"""Retain all raw artifacts/diffs, then check exact bounded migration invariants."""
import argparse, collections, copy, gzip, hashlib, json, runpy
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
legacy=runpy.run_path(str(ROOT/'docs/reports/contract-hypothesis-graph-v1/scripts/compare.py'))
differences=legacy['differences']
def erase(value):
 if isinstance(value,list):return [erase(x) for x in value]
 if not isinstance(value,dict):return value
 removed={'$hypothesis_binding_requirements','$checked_hypothesis_bindings'}
 if 'contract_sha256' in value and 'body_standing' in value:removed.add('hypothesis_binding_requirements')
 return legacy['erase']({k:erase(v) for k,v in value.items() if k not in removed})
COMPANION='detach_representation_atoms'
SELECTED='cJSON_DetachItemViaPointer'
COMPANION_LABELS={'premise:'+COMPANION, 'resource_premise('+COMPANION+')'}
p=argparse.ArgumentParser();p.add_argument('route',choices=['system','slice','corpus']);p.add_argument('base',type=Path);p.add_argument('tip',type=Path);p.add_argument('out',type=Path);a=p.parse_args()
if a.out.exists():
 assert {p.name for p in a.out.iterdir()} <= {'compile-command-correspondence.json'}, 'refuse to overwrite an existing comparison'
else:a.out.mkdir(parents=True,exist_ok=False)
inputs=ROOT/'private/premise-evidence-contracts/base-inputs'
old_source=(inputs/'providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv').read_text()
tip_inputs=ROOT/'private/premise-evidence-contracts/tip-inputs'
new_source=(tip_inputs/'providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv').read_text()
def selected_digest(root):
 lock=json.loads((root/'providers/qcore-provider-librdkafka/contracts.lock').read_text())
 records=[row for row in lock['pinned'] if row['function']==SELECTED];assert len(records)==1
 return records[0]['contract_sha256']
old_digest,new_digest=selected_digest(inputs),selected_digest(tip_inputs)
known={k:v for k,v in {new_source:old_source,new_digest:old_digest,hashlib.sha256(new_source.encode()).hexdigest():hashlib.sha256(old_source.encode()).hexdigest()}.items() if k!=v}
counts=collections.Counter()
roots=[(str(inputs),'$INPUTS'),(str(tip_inputs),'$INPUTS'),(str(ROOT),'$INPUTS'),('/home/cosmo/Documents/GitHub/qcore-c','$INPUTS')]
roots.sort(key=lambda item:len(item[0]),reverse=True)
def normalized(v):
 if isinstance(v,str):
  for root,marker in roots:v=v.replace(root,marker)
  return v
 if isinstance(v,list):return [normalized(x) for x in v]
 if isinstance(v,dict):
  pairs=[(normalized(k),normalized(x)) for k,x in v.items()];assert len(dict(pairs))==len(pairs),'key collision';return dict(pairs)
 return v
def migrate(v):
 if isinstance(v,str):
  if v in known:counts['exact_source_or_declaration_digest']+=1;return known[v]
  return v
 if isinstance(v,list):return [migrate(x) for x in v]
 if isinstance(v,dict):return {k:migrate(x) for k,x in v.items()}
 return v
def gz(path,data):path.write_bytes(gzip.compress(data,mtime=0))
names={'diagnostics':'resource-diagnostics.json','contracts':'resource-diagnostics.contracts.json','chunk-contracts':'resource-diagnostics.chunk-contracts.json','source-effects':'resource-diagnostics.source-effects.json','source':'monkey-source.json','null':a.route+'-null.jsonl','null-summary':a.route+'-null.summary.json','outer':a.route+'-outer.json'}
data={};manifest={}
for side,directory in [('base',a.base),('tip',a.tip)]:
 receipt=json.loads((directory/'invocation.json').read_text());assert receipt['exit']==0 and receipt['binary_sha256']==receipt['binary_sha256_after']
 dest=a.out/side;dest.mkdir();(dest/'invocation.json').write_text(json.dumps(receipt,indent=2)+'\n');docs={};manifest[side]={}
 for label,name in names.items():
  path=directory/name
  if not path.exists():assert label in ['source','chunk-contracts','source-effects'];continue
  raw=path.read_bytes();gz(dest/(name+'.gz'),raw);manifest[side][name]={'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest()}
  docs[label]=[json.loads(line) for line in raw.splitlines() if line] if label=='null' else json.loads(raw)
 data[side]=docs
(a.out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
raw={k:differences(data['base'][k],data['tip'][k]) for k in data['base']};gz(a.out/'raw-differences.json.gz',(json.dumps(raw,indent=2)+'\n').encode())
left={k:normalized(erase(v)) for k,v in data['base'].items()};right={k:normalized(migrate(erase(v))) for k,v in data['tip'].items()}
projected={k:differences(left[k],right[k]) for k in left};gz(a.out/'erased-migration-normalized-differences.json.gz',(json.dumps(projected,indent=2)+'\n').encode())
def without_config(units):return [{k:v for k,v in u.items() if k!='compile_configuration'} for u in units]
checks={
'static_full_units_except_separately_attributed_compile_configuration':without_config(left['diagnostics']['units'])==without_config(right['diagnostics']['units']),
'diagnostic_counts':left['diagnostics']['diagnostic_counts']==right['diagnostics']['diagnostic_counts'],
'full_null_dictionaries':left['null']==right['null'],
'null_summary':left['null-summary']==right['null-summary'],
'full_runtime_projection':left['outer']==right['outer'],
'declaration_and_clause_standing':left['contracts']['contract_dependencies']==right['contracts']['contract_dependencies'],
'contract_revision_except_exact_selected_digest':left['contracts']['contract_revision']==right['contracts']['contract_revision']}
count=lambda side:dict(collections.Counter(f['status'] for u in data[side]['diagnostics']['units'] for f in u['functions']))
expected=json.loads((ROOT/'docs/reports/premise-evidence-contracts/expected-graph.json').read_bytes())
manifest=json.loads((ROOT/'docs/reports/premise-evidence-contracts/binding-manifest.json').read_bytes())
def canonical_graph(g):
 atoms={}
 for k,a in sorted(g['atoms'].items()):
  prop=a['proposition'];assert prop['kind']=='opaque'
  atoms[k]={'id':a['id'],'proposition':{'kind':'opaque','statement':prop['statement']}}
 premises={k:{'id':p['id'],'children':p['children'],'erasure':{e:p['erasure'][e] for e in ['name','statement','origin']}} for k,p in sorted(g['premises'].items())}
 out={'version':g['version'],'atoms':atoms,'premises':premises}
 if g.get('refinements'):out['refinements']={k:{'premise':v['premise'],'children':v['children']} for k,v in sorted(g['refinements'].items())}
 return json.dumps(out,ensure_ascii=False,separators=(',',':')).encode()
assert canonical_graph(expected)==(ROOT/'docs/reports/premise-evidence-contracts/expected-graph.json').read_bytes()
graph_digest_checks=0
def verify_digests(v):
 global graph_digest_checks
 if isinstance(v,list):
  for x in v:verify_digests(x)
 elif isinstance(v,dict):
  if isinstance(v.get('hypothesis_graph'),dict):
   h=hashlib.sha256(canonical_graph(v['hypothesis_graph'])).hexdigest()
   if 'hypothesis_graph_sha256' in v:assert v['hypothesis_graph_sha256']==h;graph_digest_checks+=1
   if isinstance(v.get('hypothesis_evidence'),dict):assert v['hypothesis_evidence']['graph_sha256']==h;graph_digest_checks+=1
  if '$hypothesis_graphs' in v:
   assert v['$hypothesis_graphs'].keys()==v['$hypothesis_graph_digests'].keys()
   for name,g in v['$hypothesis_graphs'].items():assert hashlib.sha256(canonical_graph(g)).hexdigest()==v['$hypothesis_graph_digests'][name];graph_digest_checks+=1
  for x in v.values():verify_digests(x)
for docs in data.values():verify_digests(docs)
selected_graphs=[]
def graph_walk(l,r,path=()):
 if isinstance(l,dict) and isinstance(r,dict):
  if 'atoms' in l and 'premises' in l and 'version' in l:
   if r==expected:
    assert l['version']==1 and l['premises']==r['premises']
    assert all(r['atoms'][k]==v for k,v in l['atoms'].items())
    selected_graphs.append(list(path));return
   assert l==r,('unrelated graph changed',path)
   return
  for k in l.keys()&r.keys():graph_walk(l[k],r[k],path+(k,))
 elif isinstance(l,list) and isinstance(r,list):
  for i,(x,y) in enumerate(zip(l,r)):graph_walk(x,y,path+(i,))
for label in data['base']:graph_walk(data['base'][label],data['tip'][label],(label,))
assert data['base']['contracts']['contract_dependencies']['declarations'].keys()==data['tip']['contracts']['contract_dependencies']['declarations'].keys()
for name,v in data['base']['contracts']['contract_dependencies']['declarations'].items():
 assert v.get('premises',[])==data['tip']['contracts']['contract_dependencies']['declarations'][name].get('premises',[]),('legacy premise changed',name)
impact=[];unrelated=[]
for ui,(bu,tu) in enumerate(zip(data['base']['diagnostics']['units'],data['tip']['diagnostics']['units'])):
 for ri,(br,tr) in enumerate(zip(bu['functions'],tu['functions'])):
  item={'unit_index':ui,'row_index':ri,'unit':{k:tu.get(k) for k in ['library','file','label']},'function':tr['function'],'status':tr['status'],'context':tr.get('hypothesis_context')}
  bd=br.get('contract_dependencies',{});td=tr.get('contract_dependencies',{})
  bh=bd.get('hypothesis_dependencies',{});th=td.get('hypothesis_dependencies',{})
  if SELECTED in th:
   assert set(th[SELECTED])==set(expected['atoms'])
   assert set(bh[SELECTED])==set(expected['atoms'])-set(manifest['atoms'])
   assert {k:v for k,v in bh.items() if k!=SELECTED}=={k:v for k,v in th.items() if k!=SELECTED}
   item.update(consultation_dependency_record=td,declaration=SELECTED,premise='detach_list_representation',actual_atom_dependencies=th[SELECTED],manifest_sha256=__import__('hashlib').sha256((ROOT/'docs/reports/premise-evidence-contracts/binding-manifest.json').read_bytes()).hexdigest(),graph_sha256=manifest['graph_sha256'],atom_sha256=manifest['atoms']['detach_no_writer']['atom_sha256'])
   impact.append(item)
  else:
   assert bh==th
   unrelated.append(item)
selected_context=impact[0]['context'] if impact else None
if selected_context is not None:assert all(selected_context.get(k) for k in ['artifact','property','semantic_profile','scope','bounds','assumptions','source_identity','build_identity','evidence_version'])
exact_impact=[v for v in impact if v['context']==selected_context];other_contexts=[v for v in impact if v['context']!=selected_context]
impact_record={'route':a.route,'control':{'atom':'detach_no_writer','disposition':'refuted','warrant':'declared','reference':'synthetic formal writing-step control; no observed C violation'},'selected_independently_held_context':selected_context,'binding_resolution_receipt':'checked-binding-publication.json.gz','formal_refutation_target':manifest['atoms']['detach_no_writer'],'proposed_refinement':expected['refinements']['detach_list_representation'],'unchanged_legacy_node':expected['premises'][expected['refinements']['detach_list_representation']['premise']],'potential_transitive_dependents':impact,'affected_refinement_instances':exact_impact,'dependent_other_contexts_unchanged':other_contexts,'unrelated_records':unrelated,'production_verdict_changes':[],'legacy_premise_disposition_changes':[],'source_binding':'open','boundary':'Exact production graph and content-addressed binding. Refutation affects proposed conjunction only in held Context; legacy singleton stays separately Open. Actual Rust direct+summary controls establish closure/refusal. No null-property dependency is inferred.'}
gz(a.out/'impact.json.gz',(json.dumps(impact_record,indent=2)+'\n').encode())
summary={'route':a.route,'checks':checks,'base':count('base'),'tip':count('tip'),'null_rows':{side:len(data[side]['null']) for side in data},'exact_migration_transform_counts':dict(counts),'raw_differing_leaf_counts':{k:len(v) for k,v in raw.items()},'remaining_differing_leaf_counts':{k:len(v) for k,v in projected.items()},'potential_dependent_custody_records':len(impact),'affected_custody_records':len(exact_impact),'dependent_other_contexts_unchanged':len(other_contexts),'unrelated_custody_records':len(unrelated),'all_invariant_checks_pass':all(checks.values()),'verified_selected_graph_instances':selected_graphs,'fresh_graph_digest_checks':graph_digest_checks,'note':'Production same-name neutral refinement, exact original singleton/name/text and all unrelated v1 graphs retained. Every metadata residual needs separate attribution. Source binding remains Open.'}
(a.out/'comparison.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2));raise SystemExit(0 if all(checks.values()) else 1)
