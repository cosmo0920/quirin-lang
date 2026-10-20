#!/usr/bin/env python3
"""Retain all raw artifacts/diffs, then check exact bounded migration invariants."""
import argparse, collections, copy, gzip, hashlib, json, runpy
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
legacy=runpy.run_path(str(ROOT/'docs/reports/contract-hypothesis-graph-v1/scripts/compare.py'))
erase=legacy['erase']; differences=legacy['differences']
COMPANION='detach_representation_atoms'
SELECTED='cJSON_DetachItemViaPointer'
COMPANION_LABELS={'premise:'+COMPANION, 'resource_premise('+COMPANION+')'}
p=argparse.ArgumentParser();p.add_argument('route',choices=['system','slice','corpus']);p.add_argument('base',type=Path);p.add_argument('tip',type=Path);p.add_argument('out',type=Path);a=p.parse_args()
if a.out.exists():
 assert {p.name for p in a.out.iterdir()} <= {'compile-command-correspondence.json'}, 'refuse to overwrite an existing comparison'
else:a.out.mkdir(parents=True,exist_ok=False)
inputs=ROOT/'private/premise-decomposition-cycle1/base-inputs'
old_source=(inputs/'providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv').read_text()
new_source=(ROOT/'providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv').read_text()
def selected_digest(root):
 lock=json.loads((root/'providers/qcore-provider-librdkafka/contracts.lock').read_text())
 records=[row for row in lock['pinned'] if row['function']==SELECTED];assert len(records)==1
 return records[0]['contract_sha256']
old_digest,new_digest=selected_digest(inputs),selected_digest(ROOT)
known={k:v for k,v in {new_source:old_source,new_digest:old_digest,hashlib.sha256(new_source.encode()).hexdigest():hashlib.sha256(old_source.encode()).hexdigest()}.items() if k!=v}
counts=collections.Counter()
roots=[(str(inputs),'$INPUTS'),(str(ROOT),'$INPUTS'),('/home/cosmo/.codex/worktrees/w48-bottom-up-batch4/qcore-c','$INPUTS')]
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
 if isinstance(v,list):
  result=[]
  for x in v:
   if isinstance(x,dict) and x.get('name')==COMPANION:
    assert x['origin']=='declared' and x['statement'].startswith('all(detach_');counts['companion_erasure']+=1;continue
   if isinstance(x,str) and x in COMPANION_LABELS:counts['companion_clause_label']+=1;continue
   result.append(migrate(x))
  return result
 if isinstance(v,dict):
  return {k:migrate(x) for k,x in v.items() if k not in COMPANION_LABELS}
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
impact=[];unrelated=[]
for unit_index,unit in enumerate(data['tip']['diagnostics']['units']):
 for row_index,row in enumerate(unit['functions']):
  item={'unit_index':unit_index,'row_index':row_index,'unit':{k:unit.get(k) for k in ['library','file','label']},'function':row['function'],'status':row['status'],'context':row.get('hypothesis_context')}
  deps=row.get('contract_dependencies',{}).get('hypothesis_dependencies',{})
  declaration_deps=row.get('contract_dependencies',{})
  if any(SELECTED in declaration_deps.get(key,[]) for key in ['declarations','held','withdrawn']):
   item['declaration']=SELECTED;item['premise']=COMPANION;item['actual_production_atom_dependencies']=deps.get(SELECTED,[]);item['proposed_dependency_atoms']=[row['id'] for row in json.loads((ROOT/'docs/reports/premise-decomposition-cycle1/atom-map.json').read_text())['atoms']];item['attachment_binding']='open: report-owned candidate only; actual production graph retains the separate legacy atom';impact.append(item)
  else:unrelated.append(item)
null_impact=[];null_unrelated=[]
for index,row in enumerate(data['tip']['null']):
 item={'record_index':index,'library':row.get('library'),'file':row.get('file'),'function':row.get('function'),'context':row.get('hypothesis_context'),'graph':row.get('hypothesis_graph'), 'null_verdict':row.get('null',{}).get('verdict'),'combined':row.get('combined')}
 # The null property publishes its own consulted-premise graph. Do not invent
 # custody-to-null dependencies merely because both rows name one C function.
 if 'detach_no_writer' in row.get('hypothesis_graph',{}).get('atoms',{}):null_impact.append(item)
 else:null_unrelated.append(item)
selected_context=impact[0]['context'] if impact else None
if selected_context is not None:
 assert all(selected_context.get(key) for key in ['artifact','property','semantic_profile','scope','bounds','assumptions','source_identity','build_identity','evidence_version'])
exact_impact=[row for row in impact if row['context']==selected_context]
other_contexts=[row for row in impact if row['context']!=selected_context]
impact_record={'route':a.route,'control':{'atom':'detach_no_writer','disposition':'refuted','warrant':'declared','reference':'synthetic writing-step/control; no observed C violation'},'selected_control_context':selected_context,'potential_transitive_dependents':impact,'affected_report_candidate_instances':exact_impact,'production_verdict_changes':[],'production_assumption_disposition_changes':[],'dependent_other_contexts_unchanged':other_contexts,'unrelated_records':unrelated,'null_affected_records':null_impact,'null_unrelated_records':null_unrelated,'null_boundary':'The null graph is its independently consulted legacy-premise fold; no custody-to-null implication is inferred.','propagation':'Only exact selected-context report-candidate instances are affected by the hypothetical refutation; the proposed attachment is unbound and does not refute actual production legacy premises; other dependents require independent context-bound evidence. Evidence impact only; production dispositions and verdicts remain unchanged; foreign context refuses; legacy premise separately open','formal_refutation':'QCore.CjsonRepresentationPremises.a_writing_step_refutes_no_writer'}
gz(a.out/'impact.json.gz',(json.dumps(impact_record,indent=2)+'\n').encode())
summary={'route':a.route,'checks':checks,'base':count('base'),'tip':count('tip'),'null_rows':{side:len(data[side]['null']) for side in data},'exact_migration_transform_counts':dict(counts),'raw_differing_leaf_counts':{k:len(v) for k,v in raw.items()},'remaining_differing_leaf_counts':{k:len(v) for k,v in projected.items()},'potential_dependent_custody_records':len(impact),'affected_custody_records':len(exact_impact),'dependent_other_contexts_unchanged':len(other_contexts),'unrelated_custody_records':len(unrelated),'all_invariant_checks_pass':all(checks.values()),'note':'Final production module/inventory/pins are byte-identical to exact base; decomposition is report-owned. All residual leaf differences retained. Compile provenance/environment/timestamp differences require exact separate reconstruction before completion.'}
(a.out/'comparison.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2));raise SystemExit(0 if all(checks.values()) else 1)
