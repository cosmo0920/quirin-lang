"""Account the finite observed leaf set; reject any additional kind of change."""
from pathlib import Path
import collections, copy, gzip, json, re, subprocess
p=Path('private/w48-cmt-tracked-interior')
read=lambda f:json.loads(Path(f).read_bytes())
unpack=lambda f:json.loads(gzip.decompress(Path(f).read_bytes()))
expected_gains={
 'corpus':{('ctraces','ctraces.c','ctr_create',0),('monkey-libevent','mk_server/mk_stream.c','mk_channel_new',0)},
 'system':{('fluent-bit-system','src/flb_task.c','flb_task_queue_create',0),('fluent-bit-system','src/flb_task.c','flb_task_queue_create',1)},
 'slice':set()}
def at(value,path):
 for k in path:value=value[k]
 return value
def row_keys(d):
 seen=collections.Counter();answer={}
 for ui,u in enumerate(d['units']):
  for fi,r in enumerate(u['functions']):
   k=(u['library'],u['file'],r['function']);answer[ui,fi]=k+(seen[k],);seen[k]+=1
 return answer
report=[]
for route in ['corpus','system','slice']:
 out=p/'comparisons'/route;remaining=unpack(out/'remaining-differences.json.gz')
 docs={s:unpack(out/s/'resource-diagnostics.json.gz') for s in ['base','tip']}
 keys=row_keys(docs['base']);assert keys==row_keys(docs['tip'])
 moved=unpack(out/'full-row-movements.json.gz');gains={tuple(r['key']) for r in moved if r['before']['status']!=r['after']['status']}
 assert gains==expected_gains[route]
 changed={tuple(r['key']):r for r in moved}
 for r in moved:
  assert r['before']['contract_dependencies']==r['after']['contract_dependencies']
  for d in r['removed_diagnostics']:
   assert ((d['category']=='unsupported' and re.fullmatch(r'call to uncontracted function `(cfl|mk)_list_init`',d['description'])) or (d['category']=='unknown_alias' and d['description'].startswith('unresolved resource identity `'))),r
  for d in r['added_diagnostics']:
   assert d['category']=='unsupported' and d['description']=='indirect memory access is outside the resource projection',r
  if tuple(r['key']) in gains:
   assert r['before']['status']=='Unknown' and r['after']['status']=='Safe' and not r['after']['diagnostics']
  else:assert r['before']['status']==r['after']['status']=='Unknown'
 # Reconstruct all reported diagnostic category counts directly from rows.
 for side,document in docs.items():
  total=collections.Counter()
  for u in document['units']:
   counts=collections.Counter(d['category'] for r in u['functions'] for d in r['diagnostics'])
   assert dict(counts)==u['diagnostic_counts'],(route,side,u['file'])
   total.update(counts)
  assert dict(total)==document['diagnostic_counts']
 bc=unpack(out/'base/resource-diagnostics.contracts.json.gz');tc=unpack(out/'tip/resource-diagnostics.contracts.json.gz')
 expected=copy.deepcopy(bc['contract_dependencies']);declarations=expected['declarations'];roll=expected['safe_rollup']
 for k in gains:
  row=changed[k]['after'];deps=row['contract_dependencies'];assert set(deps)<= {'declarations','hypothesis_dependencies'}
  names=deps['declarations'];assert names
  for name in names:
   declarations[name]['safe_rows']+=1
   if 'hypothesis_evidence' in declarations[name]:declarations[name]['hypothesis_evidence']['dependent_safe_rows']+=1
  standings=[declarations[name]['body_standing'] for name in names]
  if all(s=='validated' for s in standings):label='resting_only_on_validated_declarations'
  elif all(s in ['validated','partly-validated'] for s in standings):label='resting_on_a_partly_validated_declaration'
  else:label='resting_on_a_declared_assumption'
  roll[label]+=1;roll['safe_rows']+=1
 assumptions=sorted(((d['safe_rows'],name) for name,d in declarations.items() if d['safe_rows']>0 and d['body_standing']=='declared-assumption'),key=lambda x:(-x[0],x[1]))[:20]
 roll['top_declared_assumptions']=[{'function':name,'safe_rows':count,**{k:declarations[name][k] for k in ['mode','status','premises']}} for count,name in assumptions]
 # Every changed dependency coordinate must equal this exact old-plus-gains fold.
 for d in remaining.get('resource-diagnostics.contracts.json',[]):
  assert d['path'][0]=='contract_dependencies'
  assert at({'contract_dependencies':expected},d['path'])==d['after'],(route,d)
 for side in ['base','tip']:
  proof=read(p/'replay-v3'/(route+'-'+side+'-projection-proof.json'))
  assert proof['complete_outer_projection_reproduced'] and proof['all_production_null_tally_fields_reproduced'] and proof['all_null_row_custody_fields_and_production_joins_reproduced']
 reviews=[]
 for filename,differences in remaining.items():
  for leaf in differences:
   path=leaf['path'];reason=None;evidence=[]
   if filename=='resource-diagnostics.json':
    if path[:1]==['units'] and len(path)>3 and path[2]=='functions':
     k=keys[path[1],path[3]];assert k in changed
     field=path[4];assert field in ['diagnostics','status']
     if field=='status':assert k in gains
     r=changed[k]
     reason=f"Rule 36b source-checked initializer transport at {k}: retained owner storage no longer receives an opaque initializer escape; actual before/after diagnostics are preserved in the full-row record. Its status is {r['before']['status']} -> {r['after']['status']}. Remaining indirect-access refusals stay Unknown. No API declared-model permission is used."
     evidence=[f'full-row-movements.json.gz key {k}','crates/qcore-c-gateway/src/legacy_tracked_self_init.rs','crates/qcore-c-gateway/src/legacy_transfer.rs:TrackedSelfInit','steps/focus-rust-3.json','steps/focus-deep-2.json']
    elif path[:1]==['diagnostic_counts'] or (path[:1]==['units'] and path[2]=='diagnostic_counts'):
     reason='Exact diagnostic category sum of the retained complete function rows; reconstructed on both sides, including all unchanged diagnostics.'
     evidence=['complete retained resource-diagnostics.json on both sides','semantic review diagnostic-count reconstruction']
    elif path[:1]==['units'] and path[2]=='reason':
     ui=path[1]
     for side,label in [('base','before'),('tip','after')]:
      c=docs[side]['units'][ui]['diagnostic_counts']
      text=f"resource projection unresolved: {c.get('unsupported',0)} unsupported operation(s), {c.get('unknown_alias',0)} unresolved alias(es), {c.get('summary_conflict',0)} custody summary conflict(s)"
      assert text==leaf[label]
     reason='Unit reason renders the exact reconstructed unsupported/alias/conflict counts; the unit remains unresolved.'
     evidence=['complete retained unit diagnostic_counts and reason','semantic review diagnostic-count reconstruction']
   elif filename=='resource-diagnostics.contracts.json':
    reason='Exact old-plus-new-Safe-rows dependency fold. Consulted declarations and standing remain unchanged. Each gained row increments its existing declarations; Safe roll-up categories are recomputed from their unchanged body standing, and top assumptions are sorted by count then name. The permutation of __errno_location/LZ4 or system assumptions is a ranking consequence, not a new premise or authority.'
    evidence=['full-row-movements.json.gz gained rows and unchanged contract_dependencies','crates/qcore-audit/src/contract_dependencies.rs:render','semantic review exact per-coordinate dependency reconstruction']
   elif filename=='null.jsonl':
    assert len(path)==2 and path[1] in ['custody','combined'];assert leaf['before']=='Unknown' and leaf['after']=='Safe'
    reason='The same gained custody row feeds the pre-existing NULL-row custody field and the production four-value combined join. Independent NULL dictionaries are unchanged; both complete production joins are replayed.'
    evidence=[f'replay-v3/{route}-base-projection-proof.json',f'replay-v3/{route}-tip-projection-proof.json','full-null-movements.json.gz (independent NULL dictionary comparison)']
   elif filename in ['null.summary.json','outer.json']:
    if filename=='null.summary.json':assert len(path)==2 and path[0] in ['custody','combined']
    else:assert path[0] in ['static','scenarios'] and path[-2] in ['custody','combined']
    assert path[-1] in ['Safe','Unknown'];assert leaf['after']-leaf['before']==(len(gains) if path[-1]=='Safe' else -len(gains))
    reason='Exact production tally/join consequence of the enumerated gained custody rows. Full static and each loaded-plugin runtime projection is reproduced separately on both sides; source binding remains Open and truth transport is false.'
    evidence=[f'replay-v3/{route}-base-projection-proof.json',f'replay-v3/{route}-tip-projection-proof.json','full-row-movements.json.gz']
   assert reason is not None,(route,filename,leaf)
   reviews.append({'leaf':{'file':filename,**leaf},'reason':reason,'evidence':evidence})
 (out/'semantic-review.json').write_text(json.dumps(reviews,indent=2)+'\n')
 report.append({'route':route,'gains':sorted(gains),'changed_resource_rows':len(moved),'semantic_leaves':len(reviews),'exact_dependency_fold_reproduced':True,'exact_diagnostic_counts_reproduced':True})
source='crates/qcore-audit/src/contract_dependencies.rs';base=read(p/'config.json')['base'];assert subprocess.check_output(['git','show',base+':'+source])==Path(source).read_bytes()
(p/'semantic-review-reconstruction.json').write_text(json.dumps({'routes':report,'dependency_producer_source_equal_semantic_base':True,'all_observed_leaf_kinds_explicitly_reviewed':True,'no_unrecognized_leaf_admitted':True},indent=2)+'\n')
print(json.dumps(report,indent=2))
