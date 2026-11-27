from pathlib import Path
from collections import Counter
import copy,gzip,json,hashlib,re
repo=Path.cwd();out=repo/'docs/reports/w48-bottom-up-batch4/artifacts/corpus';private=repo/'private/w48-bottom-up-batch4';load=lambda s,f:json.loads(gzip.decompress((out/s/(f+'.json.gz')).read_bytes()));D={s:load(s,'diagnostics') for s in ['before','after']};C={s:load(s,'contracts') for s in D};pr=json.loads((out/'context-identity-review.json').read_text());ctxkeys={'artifact','property','semantic_profile','scope','bounds','assumptions','source_identity','build_identity','evidence_version'};roots=[str(private/n) for n in ['inputs-base-final','inputs-tip-final']]
def norm(v,after=False):
 if isinstance(v,str):
  for root in roots:v=v.replace(root,'$INPUTS')
  return v
 if isinstance(v,list):return [norm(x,after) for x in v]
 if isinstance(v,dict):
  if after and set(v)==ctxkeys:
   assert v['assumptions']==pr['assumptions']['after'] and v['evidence_version']==pr['evidence_versions']['after'];v=v|{'assumptions':pr['assumptions']['before'],'evidence_version':pr['evidence_versions']['before']}
  result={norm(k):norm(x,after) for k,x in v.items()};assert len(result)==len(v);return result
 return v
def reason(unit):
 ds=[d for f in unit['functions'] for d in f.get('diagnostics',[])];c=Counter(d['category'] for d in ds);held=sum(d['category']=='unsupported' and re.fullmatch(r'call to `[^`]+` whose declared contract awaits human review \(awaiting-human-review\)',d['description']) is not None for d in ds);pins=sum(d['category']=='unsupported' and re.fullmatch(r'call to `[^`]+` whose pinned contract is not in force: the audited source does not match its lock \(contract-pin-mismatch\)',d['description']) is not None for d in ds);text=f"resource projection unresolved: {c['unsupported']-held-pins} unsupported operation(s), {c['unknown_alias']} unresolved alias(es), {c['summary_conflict']} custody summary conflict(s)";text+=f', {held} call(s) held awaiting human review' if held else '';text+=f', {pins} call(s) held for a contract pin mismatch' if pins else '';return text
a,b=norm(D['before']),norm(D['after'],True);row_changes=[];unit_changes=[]
for side in D:
 total=Counter()
 for u in D[side]['units']:
  count=Counter(d['category'] for f in u['functions'] for d in f.get('diagnostics',[]));assert count==Counter(u['diagnostic_counts']);total.update(count)
 assert total==Counter(D[side]['diagnostic_counts'])
for i,(x,y) in enumerate(zip(a['units'],b['units'])):
 assert (x['library'],x['file'])==(y['library'],y['file'])
 for side,z in [('before',x),('after',y)]:assert z['cross_unit_link_resolution']==norm(C[side]['cross_unit_link_resolution'])
 y['cross_unit_link_resolution']=x['cross_unit_link_resolution']
 for j,(left,right) in enumerate(zip(x['functions'],y['functions'])):
  assert left['function']==right['function'];fields=[k for k in left.keys()|right.keys() if left.get(k)!=right.get(k)];assert set(fields)<={'status','diagnostics','contract_dependencies','null_premises'}
  if fields:
   row_changes.append({'unit':i,'row':j,'library':x['library'],'file':x['file'],'function':left['function'],'fields':fields,'before':{k:left.get(k) for k in fields},'after':{k:right.get(k) for k in fields}})
   for row in [left,right]:
    carriers=row.get('null_premises',[]);dependencies=row.get('contract_dependencies',{}).get('null_premises',[]);assert len(dependencies)==len(set(dependencies));assert {v['name'] for v in carriers}<=set(dependencies) # dependency producer additionally closes over consulted in-unit summaries
    for v in carriers:assert v['name']==f"null-deref:{row['function']}:{v['line']}:{v['variable']}"
   for field in fields:
    if field in right:left[field]=copy.deepcopy(right[field])
    else:left.pop(field,None)
 fields=[k for k in x.keys()|y.keys() if x.get(k)!=y.get(k)];assert set(fields)<={'reason','diagnostic_counts','contract_body_evidence'}
 if 'reason' in fields:
  for side,z in [('before',x),('after',y)]:assert z['reason']==reason(D[side]['units'][i])
 if 'contract_body_evidence' in fields:
  assert x['library']=='cmetrics' and x['file']=='cmt_summary.c';name='cmt_summary_quantile_get_value';old=copy.deepcopy(x['contract_body_evidence']);new=copy.deepcopy(y['contract_body_evidence']);assert old['unvalidated'].count(name)==1 and name not in new['unvalidated'];old['unvalidated'].remove(name);assert new['validated'].pop(name)==['W4','entry'];assert new['clauses'].pop(name)=={k:['W4','entry'] for k in ['resource_borrowed(#0)','resource_borrowed(#1)','resource_return_none()']};assert old==new
 if fields:
  unit_changes.append({'unit':i,'library':x['library'],'file':x['file'],'fields':fields,'before':{k:x.get(k) for k in fields},'after':{k:y.get(k) for k in fields}})
  for field in fields:x[field]=copy.deepcopy(y[field])
a['diagnostic_counts']=b['diagnostic_counts'];assert a==b,'unattributed diagnostic artifact field'
result={'complete_diagnostic_metadata_attributed':True,'contexts':'context-identity-review.json','cross_unit_imports':'complete-contract-review.json','all_unit_and_global_diagnostic_counts_recomputed':True,'all_changed_unit_reason_strings_recomputed':True,'semantic_rows':row_changes,'semantic_units':unit_changes,'meaning':'Every residual field is retained, rather than erased: status/diagnostics, full dependency/null-carrier payloads, derived counts/reasons, and the single new cmetrics native body conformance. All other complete fields agree after individually checked context/import identities. This is not a new null-property fact or a general C source proof.'}
(out/'complete-diagnostics-review.json.gz').write_bytes(gzip.compress((json.dumps(result,indent=2)+'\n').encode(),mtime=0));print({'complete_diagnostic_metadata_attributed':True,'semantic_rows':len(row_changes),'semantic_units':len(unit_changes)})
