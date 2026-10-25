from pathlib import Path
import json,gzip,hashlib,collections,datetime,subprocess
p=Path('private/w48-cmt-tracked-interior');sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest();read=lambda f:json.loads(Path(f).read_bytes());unpack=lambda f:json.loads(gzip.decompress(Path(f).read_bytes()))
old=read(p/'config.json')['base'];new=read(p/'semantic-freeze.json')['head'];bins=[read(p/'base-binary.json')['sha256'],read(p/'tip-binary.json')['sha256']];out={};total_raw=0
for route in ['corpus','system','slice']:
 c=p/'comparisons'/route;a=read(c/'attributed-comparison.json');remaining=unpack(c/'remaining-differences.json.gz');meta=unpack(c/'measurement-provenance-differences.json.gz');manifest=read(c/'manifest.json');summary=read(c/'summary.json');total_raw+=sum(a['original_raw_difference_counts'].values())
 for side,binary,source in zip(['base','tip'],bins,[old,new]):
  inv=unpack(c/side/'producer-invocation.json.gz');assert inv['exit']==0 and inv['stable'] and inv['before']==inv['after'];assert inv['before']['binary']==binary and inv['semantic_source']==source
  assert inv['free_bytes_after']>40*1024**3
  assert datetime.datetime.fromisoformat(inv['started'])<datetime.datetime.fromisoformat(inv['ended'])
  for key in ['working_head_before','working_head_after']:assert subprocess.check_output(['git','rev-parse',inv[key]],text=True).strip()==inv[key]
  if route=='corpus':
   driver=p/'measure.py';assert inv['before']['driver']==sha(driver)
   assert inv['before']['inputs']['scripts/run_pinned_audit.py']==sha(p/'inputs/scripts/run_pinned_audit.py')
   if side=='tip':assert inv['before']['runner']==inv['before']['inputs']['scripts/run_pinned_audit.py']
   run=unpack(c/side/'run.json.gz');assert run['binary_sha256']==binary and run['source_commit']==source and run['exit']==0
   assert run['command'][0]==str((p/('base-runtime/target/release/quirin-audit' if side=='base' else 'tip-runtime/target/release/quirin-audit')).resolve())
  else:
   actual=unpack(c/side/'resource-diagnostics.invocation.json.gz');assert actual['argv'][1:]==inv['command'][1:];assert actual['reports']['report']==inv['command'][-1]
  proof=read(p/'replay-v3'/(route+'-'+side+'-projection-proof.json'))
  for key in ['complete_outer_projection_reproduced','all_production_null_tally_fields_reproduced','all_null_row_custody_fields_and_production_joins_reproduced','wrong_custody_status_negative_rejected','unprovided_null_premise_standing_negative_rejected']:assert proof[key]
  for f,h in proof['input_sha256'].items():
   spec=read(p/'replay-v3'/(route+'-'+side+'-projection-spec.json'));assert sha(spec[f])==h
 allowed_common={('after','binary'),('before','binary'),('command',0),('command',13),('ended',),('environment','QCORE_NULL_PROPERTY'),('environment','QCORE_OUTER_PROJECTION'),('free_bytes_after',),('semantic_source',),('started',),('working_head_after',),('working_head_before',)}
 for name,changes in meta.items():
  if name=='resource-diagnostics.invocation.json':allowed={('argv',13),('reports','report')}
  elif name=='run.json':allowed={('binary_sha256',),('command',0),('command',6),('command',7),('end',),('source_commit',),('start',)}
  elif route=='corpus':allowed=allowed_common|{('command',1),('command',2),('command',3)}
  else:allowed=allowed_common
  assert all(tuple(d['path']) in allowed for d in changes),(route,name,changes)
 assert not summary['safe_losses'], ('unexpected prior Safe regression', route,summary['safe_losses'])
 leaves=[{'file':name,**change} for name,changes in remaining.items() for change in changes]
 review_path=c/'semantic-review.json'
 reviews=read(review_path) if review_path.exists() else []
 assert len(leaves)==len(reviews), ('every actual remaining leaf needs explicit review',route,len(leaves),len(reviews))
 for leaf,review in zip(leaves,reviews):
  assert review['leaf']==leaf and review['reason'] and review['evidence']
 (c/'semantic-leaf-accounting.json').write_text(json.dumps({'route':route,'all_semantic_leaves_accounted':True,'unexplained_leaves':0,'leaves':reviews},indent=2)+'\n')
 a['unexplained_semantic_differences']=0;a['semantic_leaves_accounted']=len(leaves);a['all_measurement_metadata_paths_reviewed']=True;(c/'attributed-comparison.json').write_text(json.dumps(a,indent=2)+'\n');summary['attribution']='complete; zero unexplained leaves; full raw fields retained';(c/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');out[route]=dict(summary,attribution_receipt=a)
planner_equal=(p/'base-plan.json').read_bytes()==(p/'tip-plan.json').read_bytes()
result={'base':old,'measured_tip':new,'routes':out,'raw_differing_leaves_retained':total_raw,'semantic_leaves_accounted':sum(v['attribution_receipt']['semantic_leaves_accounted'] for v in out.values()),'unexplained_leaves':0,'reading_receipts_replayed':sum(v['attribution_receipt']['complete_reading_receipts_replayed'] for v in out.values()),'captured_environment_entries_validated':sum(v['attribution_receipt']['complete_canonical_captures_validated'] for v in out.values()),'null_dictionaries_compared':sum(v['null_rows'] for v in out.values()),'null_dictionaries_changed':sum(v['complete_null_dictionary_changes'] for v in out.values()),'no_prior_safe_loss':True,'planner_exactly_equal':planner_equal,'planner_sha256':sha(p/'tip-plan.json'),'source_binding':'Open','truth_transport':False,'unsafe_qualification':'provisional labels only; never upstream defect claims'}
(p/'native-comparison.json').write_text(json.dumps(result,indent=2)+'\n');print({k:v for k,v in result.items() if k!='routes'})

