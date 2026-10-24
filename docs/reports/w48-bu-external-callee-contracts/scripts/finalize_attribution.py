"""Account for every artifact field by exact preimages and bounded observed semantic changes.
No transformed verdict is published; original artifacts remain authoritative.
"""
from pathlib import Path
import argparse,json,hashlib,copy,collections,subprocess
ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--evidence-root',type=Path,default=Path(__file__).parent);ap.add_argument('--routes',nargs='+',default=['corpus','system','slice']);a=ap.parse_args();P=a.evidence_root.resolve()
read=lambda f:json.loads(f.read_bytes());sha=lambda b:hashlib.sha256(b).hexdigest();canon=lambda v:json.dumps(v,sort_keys=True,ensure_ascii=False,separators=(',',':'))
newdeps={('librdkafka','tinycthread_extra.c','thrd_current_id'):'thrd_current',('monkey-libevent','mk_core/mk_iov.c','mk_iov_send'):'writev',('monkey-libevent','mk_server/mk_stream.c','mk_channel_stream_write'):'writev',('monkey-libevent','mk_server/mk_stream.c','mk_channel_write'):'writev'}
targets={('librdkafka','tinycthread_extra.c','thrd_current_id'),('monkey-libevent','mk_core/mk_iov.c','mk_iov_send')}
for route in a.routes:
 dirs={s:P/('base-3'if s=='base'else'tip-1')/route for s in ['base','tip']};rd={s:d/('corpus'if route=='corpus'else'')for s,d in dirs.items()};gain=2 if route=='corpus'else 0
 ids=read(P/(route+'-identity-attribution.json'));rp=read(P/(route+'-reading-attribution.json'));tp=read(P/(route+'-time-key-attribution.json'));mp=read(P/(route+'-contract-metadata-attribution.json'));assert all(v['exit']==0 for v in [ids,rp,tp,mp])
 pairs={(r['tip'],f):r['base']for r in ids['exact_field_scoped_pairs']for f in r['allowed_fields']}
 def inverse(v,path=()):
  if isinstance(v,list):return [inverse(x,path+(i,))for i,x in enumerate(v)]
  if isinstance(v,dict):return {k:inverse(x,path+(k,))for k,x in v.items()}
  if isinstance(v,str)and path and (v,path[-1])in pairs:return pairs[(v,path[-1])]
  return v
 docs={s:read(d/'resource-diagnostics.json')for s,d in rd.items()}
 for side in dirs:assert rp['input_diagnostics_sha256'][side]==sha((rd[side]/'resource-diagnostics.json').read_bytes())
 restored=copy.deepcopy(docs['tip']);envmaps=collections.defaultdict(dict);receipts=collections.defaultdict(dict)
 for r in rp['environment_records']:envmaps[r['unit']][r['tip']]=r['base']
 for r in rp['receipt_records']:receipts[(r['unit'],r['function'])][r['tip']]=r['base']
 for ui,(b,t)in enumerate(zip(docs['base']['units'],restored['units'])):
  if 'reading_site_environments'in t:
   actual=t['reading_site_environments'];base=b['reading_site_environments'];mapped={}
   for h,value in actual['environments'].items():
    assert sha(value.encode())==h;bh=envmaps[ui][h];assert sha(base['environments'][bh].encode())==bh;mapped[bh]=base['environments'][bh]
   assert set(mapped)==set(base['environments']);actual['environments']=mapped
  br={r['function']:r for r in b['functions']}
  for row in t['functions']:
   if 'reading_site_evidence'not in row:continue
   baseline={r['id']:r for r in br[row['function']]['reading_site_evidence']};mapping=receipts[(ui,row['function'])];assert len(mapping)==len(row['reading_site_evidence'])
   row['reading_site_evidence']=sorted([copy.deepcopy(baseline[mapping[r['id']]])for r in row['reading_site_evidence']],key=lambda r:r['id'])
 restored=inverse(restored);expected=copy.deepcopy(docs['base']);dep_records=[];moved=[]
 if gain:
  for u in expected['units']:
   changed=False
   for r in u['functions']:
    key=(u['library'],u['file'],r['function'])
    if key in newdeps:
     name=newdeps[key];old=r['contract_dependencies']['declarations'];assert name not in old;dep_records.append({'artifact':key,'base':old.copy(),'tip':sorted(old+[name])});r['contract_dependencies']['declarations']=sorted(old+[name])
    if key in targets:
     name=newdeps[key];assert r['status']=='Unknown'and len(r['diagnostics'])==1 and r['diagnostics'][0]['description']==f'call to uncontracted function `{name}`';r['status']='Safe';r['diagnostics']=[];changed=True;moved.append(key)
   if changed:
    counts=u['diagnostic_counts'];counts['unsupported']-=1
    u['reason']=f"resource projection unresolved: {counts['unsupported']} unsupported operation(s), {counts.get('unknown_alias',0)} unresolved alias(es), {counts.get('summary_conflict',0)} custody summary conflict(s)"
  assert set(moved)==targets and len(dep_records)==4;expected['diagnostic_counts']['unsupported']-=gain
 assert restored==expected,(route,'unexplained diagnostics/reading/projection field')
 cb,ct=[read(rd[s]/'resource-diagnostics.contracts.json')for s in ['base','tip']]
 for actual in [cb,ct]:
  for node in actual['contract_dependencies']['declarations'].values():
   contexts=node.get('hypothesis_evidence',{}).get('contexts',[])
   assert contexts==sorted(contexts,key=canon) and len(contexts)==len({canon(c)for c in contexts})
 order_changed=[]
 order_source='crates/qcore-audit/src/contract_dependencies.rs';order_bytes=[subprocess.check_output(['git','show',ids['commits'][side]+':'+order_source])for side in ['base','tip']];assert order_bytes[0]==order_bytes[1]

 for side in dirs:assert mp['record']['input_metadata_sha256'][side]==sha((rd[side]/'resource-diagnostics.contracts.json').read_bytes())
 cm=copy.deepcopy(ct);source_key='sources'if route=='corpus'else'contracts'
 # These whole subtrees already have complete exact independent reconstructions.
 for k in ['contract_revision',source_key]+(['contract_premises']if'contract_premises'in cb else[]):cm[k]=copy.deepcopy(cb[k])
 for r in tp['moves']:
  node=cm
  for k in r['path']:node=node[k]
  assert node.pop(r['tip'])=={}and r['base']not in node;node[r['base']]={}
 new_records=[]
 for name in ['writev','thrd_current']:
  if name not in ct['contract_dependencies']['declarations']:continue
  assert name not in cb['contract_dependencies']['declarations'];actual=ct['contract_dependencies']['declarations'][name];rows=[r for u in docs['tip']['units']for r in u['functions']if name in r.get('contract_dependencies',{}).get('declarations',[])];assert rows
  contexts=sorted({canon(r['hypothesis_context']):r['hypothesis_context']for r in rows}.values(),key=canon);safe=sum(r['status']=='Safe'for r in rows);rev=ct['contract_revision']['contracts'][name];graph={'version':1,'atoms':{},'premises':{}};graph_digest=sha(json.dumps(graph,separators=(',',':')).encode());template=ct['contract_dependencies']['declarations']['write']
  expected_node={'body_standing':'declared-assumption','contract_sha256':rev['contract_sha256'],'hypothesis_evidence':{'contexts':contexts,'dependent_rows':len(rows),'dependent_safe_rows':safe,'disposition':'open','graph_sha256':graph_digest,'meaning':template['hypothesis_evidence']['meaning'],'standing':'declared-assumption','warrant':'declared'},'hypothesis_graph':graph,'mode':'flexible','module':rev['module'],'premises':[],'rows':len(rows),'safe_rows':safe,'status':'flexible-standing'}
  assert actual==expected_node,(route,name,'unexplained new declaration evidence');new_records.append({'function':name,'rows':len(rows),'safe_rows':safe,'context_artifacts':[c['artifact']for c in contexts],'full_record_reconstructed':True});del cm['contract_dependencies']['declarations'][name]
 cm=inverse(cm)
 for name,node in cm['contract_dependencies']['declarations'].items():
  contexts=node.get('hypothesis_evidence',{}).get('contexts')
  if contexts is not None:
   ordered=sorted(contexts,key=canon)
   if ordered!=contexts:order_changed.append(name)
   node['hypothesis_evidence']['contexts']=ordered
 if gain:
  for k in ['declarations_consulted','declared_assumption']:cm['contract_dependencies']['counts'][k]-=len(new_records)
  for k in ['safe_rows','resting_on_a_declared_assumption']:cm['contract_dependencies']['safe_rollup'][k]-=gain
 assert cm==cb,(route,'unexplained full contract metadata field')
 null={s:[json.loads(l)for l in(d/(route+'-null.jsonl')).read_text().splitlines()]for s,d in dirs.items()};assert len(null['base'])==len(null['tip']);null_moved=[]
 for b,t in zip(null['base'],null['tip']):
  assert (b['library'],b['file'],b['function'])==(t['library'],t['file'],t['function']);assert b['null']==t['null'];expected=copy.deepcopy(b);key=(b['library'],b['file'],b['function'])
  if gain and key in targets:
   assert b['custody']==b['combined']=='Unknown'and t['custody']==t['combined']=='Safe';expected['custody']=expected['combined']='Safe';null_moved.append({'artifact':key,'null_scope':t['null']['scope'],'null_assumptions':t['null']['assumptions']})
  assert inverse(t)==expected,(route,key,'unexplained full null-record field')
 assert len(null_moved)==gain
 for name in [route+'-null.summary.json',route+'-outer.json']:
  b,t=[read(d/name)for d in dirs.values()];expected=copy.deepcopy(b)
  if gain:
   blocks=[expected]if name.endswith('summary.json')else[expected['static'],*expected['scenarios'].values()]
   for block in blocks:
    for k in ['custody','combined']:block[k]['Safe']+=gain;block[k]['Unknown']-=gain
  assert t==expected,(route,name,'unexplained property fold or outer scenario field')
 unchanged=[]
 for name in ['monkey-source.json','resource-diagnostics.source-effects.json','resource-diagnostics.chunk-contracts.json']:
  bf,tf=rd['base']/name,rd['tip']/name;assert bf.exists()==tf.exists()
  if bf.exists():assert bf.read_bytes()==tf.read_bytes();unchanged.append(name)
 summary=read(P/'comparison-1'/route/'summary.json');assert len(summary['status_changes'])==gain and len(summary['diagnostic_trade_rows']if isinstance(summary['diagnostic_trade_rows'],list)else[None]*summary['diagnostic_trade_rows'])==gain
 result={'route':route,'exit':0,'all_json_fields_accounted':True,'unexplained_fields':0,'fields_erased':0,'fields_masked':0,'actual_status_changes':summary['status_changes'],'actual_diagnostic_trade_rows':summary['diagnostic_trade_rows'],'new_declaration_dependency_annotations':dep_records,'new_declaration_evidence_records':new_records,'context_array_order':{'producer':order_source,'source_sha256':sha(order_bytes[0]),'rule':'Facts::absorb stores Value::to_string in BTreeSet; both actual arrays checked canonical and unique; reapply the same ordering after exact build-identity preimage reversal. Every context retained.','declarations_reordered':order_changed},'combined_moves_with_unchanged_null_scopes':null_moved,'unchanged_byte_artifacts':unchanged,'raw_difference_counts':summary['raw_difference_counts'],'effective_environment_captures_reconstructed':rp['effective_environment_captures'],'typed_receipt_id_preimages_replayed':rp['receipts'],'empty_summary_key_moves':tp['moves'],'independent_proof_sha256':{n:sha((P/(route+'-'+n+'.json')).read_bytes())for n in ['identity-attribution','reading-attribution','time-key-attribution','contract-metadata-attribution']},'input_artifacts':summary['artifact_receipts'],'meaning':'Every raw field retained and accounted by bounded exact producer preimages, reviewed two-declaration/source changes, and the enumerated observed semantics. No inverse comparison verdict is published; actual tuples and artifacts remain authoritative. C extraction, libc implementation behavior and Rust-Lean correspondence remain open declared boundaries.'}
 (P/(route+'-complete-attribution.json')).write_text(json.dumps(result,indent=2)+'\n');print({'route':route,'all_fields_accounted':True,'status_moves':gain,'receipts':rp['receipts'],'unexplained':0})
