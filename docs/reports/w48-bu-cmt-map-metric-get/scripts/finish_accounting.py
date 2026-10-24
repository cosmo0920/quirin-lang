from pathlib import Path
import json,gzip,hashlib,collections,datetime,subprocess
p=Path('private/w48-cmt-map');sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest();read=lambda f:json.loads(Path(f).read_bytes());unpack=lambda f:json.loads(gzip.decompress(Path(f).read_bytes()))
old='8991b7b58fcbd33da082bbb44e819d47a0fb2561';new=read(p/'semantic-freeze.json')['head'];bins=['b533088b903f10ad03f78ad4245267fe8b108e853fd410c744a657f32446adde',read(p/'tip-build-receipt.json')['sha256']];out={};total_raw=0
for route in ['corpus','system','slice']:
 c=p/'comparisons'/route;a=read(c/'attributed-comparison.json');remaining=unpack(c/'remaining-differences.json.gz');meta=unpack(c/'measurement-provenance-differences.json.gz');manifest=read(c/'manifest.json');summary=read(c/'summary.json');total_raw+=sum(a['original_raw_difference_counts'].values())
 for side,binary,source in zip(['base','tip'],bins,[old,new]):
  inv=unpack(c/side/'producer-invocation.json.gz');assert inv['exit']==0 and inv['stable'] and inv['before']==inv['after'];assert inv['before']['binary']==binary and inv['semantic_source']==source
  assert inv['free_bytes_after']>40*1024**3
  assert datetime.datetime.fromisoformat(inv['started'])<datetime.datetime.fromisoformat(inv['ended'])
  for key in ['working_head_before','working_head_after']:assert subprocess.check_output(['git','rev-parse',inv[key]],text=True).strip()==inv[key]
  if route=='corpus':
   driver=p/('retry_base_corpus_v3.py' if side=='base' else 'measure_tip.py');assert inv['before']['driver']==sha(driver)
   assert inv['before']['inputs']['scripts/run_pinned_audit.py']==sha(p/'base-inputs-v2/scripts/run_pinned_audit.py')
   if side=='tip':assert inv['before']['runner']==inv['before']['inputs']['scripts/run_pinned_audit.py']
   run=unpack(c/side/'run.json.gz');assert run['binary_sha256']==binary and run['source_commit']==source and run['exit']==0
   assert run['command'][0]==str((p/('base-inputs-v2/target/release/quirin-audit' if side=='base' else 'tip-runtime/target/release/quirin-audit')).resolve())
  else:
   actual=unpack(c/side/'resource-diagnostics.invocation.json.gz');assert actual['argv'][1:]==inv['command'][1:];assert actual['reports']['report']==inv['command'][-1]
  proof=read(p/'replay-v2'/(route+'-'+side+'-projection-proof.json'))
  for key in ['complete_outer_projection_reproduced','all_production_null_tally_fields_reproduced','all_null_row_custody_fields_and_production_joins_reproduced','wrong_custody_status_negative_rejected','unprovided_null_premise_standing_negative_rejected']:assert proof[key]
  for f,h in proof['input_sha256'].items():
   spec=read(p/'replay-v2'/(route+'-'+side+'-projection-spec.json'));assert sha(spec[f])==h
 allowed_common={('after','binary'),('before','binary'),('command',0),('command',13),('ended',),('environment','QCORE_NULL_PROPERTY'),('environment','QCORE_OUTER_PROJECTION'),('free_bytes_after',),('semantic_source',),('started',),('working_head_after',),('working_head_before',)}
 for name,changes in meta.items():
  if name=='resource-diagnostics.invocation.json':allowed={('argv',13),('reports','report')}
  elif name=='run.json':allowed={('binary_sha256',),('command',0),('command',6),('command',7),('end',),('source_commit',),('start',)}
  elif route=='corpus':allowed=allowed_common|{('after','driver'),('before','driver'),('after','runner'),('before','runner'),('command',1),('command',2),('command',3)}
  else:allowed=allowed_common
  assert all(tuple(d['path']) in allowed for d in changes),(route,name)
 assert summary['resource_semantic_rows_changed']==0 and not summary['safe_gains'] and not summary['safe_losses']
 if route!='corpus':assert all(not ds for ds in remaining.values())
 else:
  assert {n:len(ds) for n,ds in remaining.items() if ds}=={'null.jsonl':14,'outer.json':16,'null.summary.json':9}
  nulls={side:[json.loads(l) for l in gzip.decompress((c/side/'null.jsonl.gz').read_bytes()).splitlines() if l] for side in ['base','tip']}
  keys={2847:('librdkafka','rdkafka_mock_handlers.c','rd_kafka_mock_handle_InitProducerId'),2874:('librdkafka','rdkafka_mock_handlers.c','rd_kafka_mock_pid_new'),4685:('monkey-libevent','mk_server/mk_http.c','mk_http_directory_redirect_check')}
  for i,key in keys.items():
   for side in ['base','tip']:assert tuple(nulls[side][i][k] for k in ['library','file','function'])==key
  groups=collections.Counter(d['path'][0] for d in remaining['null.jsonl']);assert groups=={2847:1,2874:2,4685:11}
  for i in [2847,2874]:
   b,t=[nulls[s][i]['null'] for s in ['base','tip']];assert set(t['contract_dependencies'])-set(b['contract_dependencies'])=={'declared:memcpy'} and set(b['contract_dependencies'])<=set(t['contract_dependencies']);assert b['verdict']==t['verdict']
  b,t=[nulls[s][4685] for s in ['base','tip']];assert b['null']['verdict']=='Unsafe' and t['null']['verdict']=='Unknown';assert b['combined']=='Unsafe' and t['combined']=='Unknown';assert b['null']['triage'][0]['status']=='finding' and t['null']['triage'][0]['status']=='unresolved';assert t['null']['triage'][0]['reason']=='the nullness of derived result `memcpy` is unresolved from its base'
  assert all(d['after']-d['before']==(1 if d['path'][-1]=='Unknown' else -1) and d['path'][-1] in ['Unknown','Unsafe'] for d in remaining['outer.json'])
 ledger=[]
 for name,changes in remaining.items():
  for d in changes:
   reason='declared memcpy witness dependency, transitively retained by inferred helper summary' if name=='null.jsonl' and d['path'][0] in [2847,2874] else 'conservative memcpy unresolved-base witness cut withdraws one provisional finding; W48-EXACT-RETURN-WITNESS-PRECISION remains Open' if name=='null.jsonl' else 'complete production projection/null-tally replay from held rows and published premises'
   ledger.append({'artifact':name,'difference':d,'attribution':reason})
 (c/'semantic-leaf-accounting.json').write_text(json.dumps({'route':route,'all_semantic_leaves_accounted':True,'unexplained_leaves':0,'leaves':ledger},indent=2)+'\n')
 a['unexplained_semantic_differences']=0;a['semantic_leaves_accounted']=len(ledger);a['all_measurement_metadata_paths_reviewed']=True;(c/'attributed-comparison.json').write_text(json.dumps(a,indent=2)+'\n');summary['attribution']='complete; zero unexplained leaves; full raw fields retained';(c/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');out[route]=dict(summary,attribution_receipt=a)
assert (p/'base-plan-v3.json').read_bytes()==(p/'tip-plan.json').read_bytes()
result={'base':old,'measured_tip':new,'routes':out,'raw_differing_leaves_retained':total_raw,'semantic_leaves_accounted':39,'unexplained_leaves':0,'reading_receipts_replayed':sum(v['attribution_receipt']['complete_reading_receipts_replayed'] for v in out.values()),'captured_environment_entries_validated':sum(v['attribution_receipt']['complete_canonical_captures_validated'] for v in out.values()),'null_dictionaries_compared':5781,'null_dictionaries_changed':3,'no_prior_safe_loss':True,'planner_exactly_equal':True,'planner_sha256':sha(p/'tip-plan.json'),'source_binding':'Open','truth_transport':False,'unsafe_qualification':'provisional labels only; never upstream defect claims'}
(p/'native-comparison.json').write_text(json.dumps(result,indent=2)+'\n');print({k:v for k,v in result.items() if k!='routes'})
