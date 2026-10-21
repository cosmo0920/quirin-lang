#!/usr/bin/env python3
"""Close each retained semantic leaf using full producer/source receipts."""
import collections,gzip,hashlib,json,re
from pathlib import Path
root=Path.cwd();p=root/'private/w48-batch5';read=lambda x:json.loads(Path(x).read_bytes());sha=lambda x:hashlib.sha256(Path(x).read_bytes()).hexdigest();results={}
for route in ['system','slice','corpus']:
 out=p/'comparisons'/route;stage=p/'analysis-inputs'/route;docs={s:read(stage/s/'resource-diagnostics.json') for s in ['base','tip']};remaining=json.loads(gzip.decompress((out/'identity-review/remaining-differences.json.gz').read_bytes()));review=[]
 for side in ['base','tip']:
  for copied in read(stage/side/'copy-manifest.json'):assert sha(copied['source'])==sha(copied['copy'])==copied['sha256']
  total=collections.Counter()
  for unit in docs[side]['units']:
   own=collections.Counter(d['category'] for r in unit['functions'] for d in r.get('diagnostics',[]));assert dict(own)==unit['diagnostic_counts'];total.update(own)
  assert dict(total)==docs[side]['diagnostic_counts']
  metadata=read(out/('metadata-'+side+'.json'));assert metadata['complete_dependency_metadata_reproduced'] and metadata['wrong_status_negative_rejected'] and metadata['wrong_build_context_negative_rejected'] and metadata['unprovided_body_method_negative_rejected']
  projection=read(out/('projection-'+side+'.json'));assert projection['complete_outer_projection_reproduced'] and projection['all_production_null_tally_fields_reproduced'] and projection['all_null_row_custody_fields_and_production_joins_reproduced'];assert projection['wrong_custody_status_negative_rejected'] and projection['unprovided_null_premise_standing_negative_rejected']
  for k,name in [('diagnostics','resource-diagnostics.json'),('contracts','resource-diagnostics.contracts.json'),('null',route+'-null.jsonl'),('null_summary',route+'-null.summary.json'),('outer',route+'-outer.json')]:assert projection['input_sha256'][k]==sha(stage/side/name)
 proof=read(out/'reading-proof.json');assert proof['input_sha256']['base_diagnostics']==sha(stage/'base/resource-diagnostics.json');assert proof['input_sha256']['tip_diagnostics']==sha(stage/'tip/resource-diagnostics.json')
 attr=read(out/'raw/source-body-attribution.json');assert all(not b['prior_validated_payloads_changed'] for b in attr['body_evidence_changes'])
 time=read(p/'time-inputs'/route/'time-identity-review.json');assert time.get('complete_summary_payloads_identical')or time['wrong_time_negative_control']['rejected'];time_keys={time.get('before_key'),time.get('after_key')}
 for family,leaves in remaining.items():
  for leaf in leaves:
   path=leaf['path'];kind=None
   if family=='diagnostics':
    if path[0]=='diagnostic_counts':kind='exact tally of complete retained row diagnostics'
    elif path[0]=='units':
     index=path[1];before=docs['base']['units'][index];after=docs['tip']['units'][index]
     if path[2]=='diagnostic_counts':kind='exact unit tally of complete retained row diagnostics'
     elif path[2]=='reason':
      old=before['reason'];expected=old;keys={'unsupported operation(s)':'unsupported','unresolved alias(es)':'unknown_alias','custody conflict(s)':'custody_conflict'}
      for label,key in keys.items():
       pattern=r'(\d+) '+re.escape(label);delta=after['diagnostic_counts'].get(key,0)-before['diagnostic_counts'].get(key,0)
       expected=re.sub(pattern,lambda m:str(int(m.group(1))+delta)+' '+label,expected)
      assert expected==after['reason'];kind='exact unchanged reason text plus independently recounted diagnostic deltas'
     elif path[2]=='contract_body_evidence':
      change=next(c for c in attr['body_evidence_changes'] if c['index']==index);assert change['new_replay_payloads'];kind='complete native body-standing change retained; exact dependency producer replay'
     elif path[2]=='reading_site_environments' or len(path)>4 and path[2]=='functions' and path[4]=='reading_site_evidence':
      assert route=='corpus';semantic=[r for r in proof['receipts'] if r.get('semantic_function_changes')];assert semantic and all(c['function']in ['cio_chunk_is_file','cio_version'] for r in semantic for c in r['semantic_function_changes']);kind='full original canonical/bind replay; real source-Safe inferred table payloads retained'
     elif path[2]=='functions':
      old=before['functions'][path[3]];new=after['functions'][path[3]];assert old['function']==new['function'];field=path[4]
      if field in ['status','diagnostics']:
       match=[a for a in attr['rows'] if a['key'][:3]==[before['library'],before['file'],old['function']]];assert match and all(a['source_observations'] for a in match);assert old['status']!='Safe' or new['status']=='Safe';kind='actual scalar read or existing summary propagation, full C coordinates retained'
      elif field=='null_premises' or field=='contract_dependencies' and path[5]=='null_premises':
       a=old.get('null_premises',[]);b=new.get('null_premises',[]);assert all(x in b for x in a);assert new['contract_dependencies']['null_premises']==[x['name'] for x in b];assert any(c['unit']==index and c['row']==path[3] for c in attr['named_null_premise_changes']);kind='additional checked scalar-read named premise; no null truth discharge'
      elif field=='contract_dependencies' and path[5]=='summaries':
       a=old['contract_dependencies'].get('summaries',[]);b=new['contract_dependencies'].get('summaries',[]);assert set(a)<=set(b)
       for name in set(b)-set(a):assert any(r['function']==name and r['status']=='Safe' and not r['diagnostics'] for r in after['functions'])
       kind='existing source-Safe summary consultation, including propagated new scalar bodies'
   elif family=='contracts':
    if path[:1]==['contract_dependencies']:kind='complete production Facts/premises/render replay both sides with three negative controls'
    elif path[:2]==['callback_members','lifetime_protocols']or path[:2]==['callback_members','prepass']:
     assert path[-1]in time_keys and leaf.get('before')in [{},None]and leaf.get('after')in [{},None];kind='exact empty-summary __TIME__ preimage replay with wrong-time negative'
   elif family=='source-effects':
    receipt=read(out/'source-effect-replay.json');assert receipt['status']=='complete source-effects identity/task/witness replay exact'and all(r['entire_normalized_record_exact'] for r in receipt['records']);kind='complete original source-effect records/task IDs/witnesses replayed; sole held cwd identity delta'
   elif family=='null':
    assert path[1]in ['custody','combined','custody_null_premises'];kind='exact production custody/null join; new named premises remain candidate'
   elif family=='null-summary':kind='exact production null-tally field replay; supplemental fold fields unchanged'
   elif family=='outer':kind='complete production outer source/pin/reach/static/runtime JSON replay both sides'
   assert kind,(route,family,path)
   review.append({'family':family,'path':path,'attribution':kind})
 counts=collections.Counter(r['attribution'] for r in review);result={'route':route,'leaf_count':len(review),'all_leaves_accounted_for':True,'remaining_unexplained':0,'classification_counts':dict(counts),'fields_erased':0,'source_binding':'Open','truth_transport':False,'reading_proof_sha256':sha(out/'reading-proof.json'),'full_private_leaf_review':str(out/'semantic-leaf-review.json')};(out/'semantic-leaf-review.json').write_text(json.dumps({'summary':result,'leaves':review},indent=2)+'\n');results[route]=result
(p/'semantic-review-summary.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps(results,indent=2))
