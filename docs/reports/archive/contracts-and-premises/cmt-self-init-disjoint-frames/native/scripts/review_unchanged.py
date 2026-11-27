"""Strict zero-semantic-delta branch; any actual delta requires separate review."""
from pathlib import Path
import collections,gzip,json,subprocess,hashlib
p=Path('private/cmt-disjoint-frames')
read=lambda f:json.loads(Path(f).read_bytes())
unpack=lambda f:json.loads(gzip.decompress(Path(f).read_bytes()))
reports=[]
for route in ['corpus','system','slice']:
 out=p/'comparisons'/route;summary=read(out/'summary.json');remaining=unpack(out/'remaining-differences.json.gz');attribution=read(out/'attributed-comparison.json')
 assert not summary['safe_gains'] and not summary['safe_losses'], 'not the zero-verdict-change case: retain and explicitly attribute actual gains/losses'
 assert not unpack(out/'full-row-movements.json.gz'), 'resource diagnostics/dependencies changed; requires actual row review'
 assert not unpack(out/'full-null-movements.json.gz'), 'independent NULL dictionary changed; requires explicit review'
 assert all(not leaves for leaves in remaining.values()), 'unexplained remaining leaves: do not discard or normalize them'
 assert attribution['fields_erased']==0 and attribution['source_binding']=='Open' and attribution['truth_transport'] is False
 for side in ['base','tip']:
  d=unpack(out/side/'resource-diagnostics.json.gz');total=collections.Counter()
  for u in d['units']:
   counts=collections.Counter(x['category'] for r in u['functions'] for x in r['diagnostics']);assert dict(counts)==u['diagnostic_counts'];total.update(counts)
  assert dict(total)==d['diagnostic_counts']
  proof=read(p/'replay-v3'/(route+'-'+side+'-projection-proof.json'))
  for key in ['complete_outer_projection_reproduced','all_production_null_tally_fields_reproduced','all_null_row_custody_fields_and_production_joins_reproduced','wrong_custody_status_negative_rejected','unprovided_null_premise_standing_negative_rejected']:assert proof[key] is True
  assert proof['source_binding']=='Open' and proof['truth_transport'] is False
 reports.append({'route':route,'resource_rows':summary['resource_rows'],'base_statuses':summary['base_statuses'],'tip_statuses':summary['tip_statuses'],'resource_semantic_rows_changed':0,'independent_null_dictionary_changes':0,'remaining_semantic_leaves':0,'complete_diagnostic_counts_reproduced':True,'both_full_production_projection_replays_passed':True,'negative_controls_rejected':True,'complete_raw_artifacts_retained':True,'source_binding':'Open','truth_transport':False})
source='crates/qcore-audit/src/contract_dependencies.rs';base=read(p/'config.json')['base'];assert subprocess.check_output(['git','show',base+':'+source])==Path(source).read_bytes()
r={'routes':reports,'dependency_producer_source_equal_semantic_base':True,'forecast_zero_production_safe_gain_matched':True,'all_observed_leaf_kinds_explicitly_reviewed':True,'no_unrecognized_leaf_admitted':True,'qualification':'Only the strict zero-semantic-delta branch. Raw evidence versions, complete receipt rebinding, exact date/time keys and measurement provenance remain separately retained; no source truth transport.'}
(p/'semantic-review-reconstruction.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))
