"""Publish only a completed, strictly unchanged paired measurement."""
from pathlib import Path
import json,gzip,hashlib,shutil
p=Path('private/cmt-disjoint-frames');r=Path('docs/reports/cmt-self-init-disjoint-frames')
read=lambda f:json.loads(Path(f).read_bytes())
assert (p/'tip-campaign.exit').read_text().strip()=='0'
review=read(p/'semantic-review-reconstruction.json');assert review['forecast_zero_production_safe_gain_matched']
assert len(review['routes'])==3
out=r/'native';out.mkdir(exist_ok=False)
shutil.copytree(p/'comparisons',out/'comparisons')
shutil.copytree(p/'replay-v3',out/'replay',ignore=shutil.ignore_patterns('rebind-readings','projection-replay','base','tip','*.rlib'))
for f in [p/'semantic-review-reconstruction.json',p/'production-targets.json',p/'semantic-freeze.json',p/'planner-differences.json.gz']:
 shutil.copyfile(f,out/f.name)
scriptdir=out/'scripts';scriptdir.mkdir()
for name in ['measure.py','measure_configured.py','retain_compare.py','rebind_readings.rs','projection_replay.rs','post_attribution.py','time_identity_v2.py','attribute_compare.py','review_unchanged.py','production_targets.py','publish_unchanged.py','finish_attribution.py','prepare_amendments_unchanged.py']:
 shutil.copyfile(p/name,scriptdir/name)
receipts={};manifest={}
for side in ['base','tip']:
 receipts[side]={}
 assert (p/(side+'-campaign.exit')).read_text().strip()=='0'
 for route in ['corpus','system','slice']:
  path=p/(side+'-corpus-invocation.json') if route=='corpus' else p/(side+'-'+route)/'invocation.json'
  d=read(path);assert d['exit']==0 and d['stable'] and d['before']==d['after']
  receipts[side][route]=d
 for suffix in ['slot.json','plan-invocation.json','plan.json','plan.log','corpus-driver.log','system-driver.log','slice-driver.log']:
  f=p/(side+'-'+suffix);assert f.exists(),f
  (out/(f.name+'.gz')).write_bytes(gzip.compress(f.read_bytes(),mtime=0))
rows=[read(out/'comparisons'/route/'summary.json') for route in ['corpus','system','slice']]
receipt={'schema':'cmt-self-init-disjoint-native/1','semantic_base':read(p/'config.json')['base'],'semantic_tip':read(p/'semantic-freeze.json')['head'],'prediction_commits':['acb13489c','b593500d9'],'routes':rows,'invocations':receipts,'resource_safe_gain':0,'resource_safe_loss':0,'getter_closure_gain':read(p/'production-targets.json')['getter_closure_gain'],'all_input_before_after_hashes_stable':True,'all_raw_artifacts_retained':True,'source_binding':'Open','truth_transport':False,'qualification':'Strict zero semantic difference branch after complete receipt/evidence-version and exact date/time identity reconstruction; configured historical evidence remains Unknown; Unsafe remains provisional.'}
(out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
for f in sorted(out.rglob('*')):
 if f.is_file():manifest[str(f.relative_to(out))]={'bytes':f.stat().st_size,'sha256':hashlib.sha256(f.read_bytes()).hexdigest()}
(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps({'published_files':len(manifest),'bytes':sum(x['bytes'] for x in manifest.values()),'rows':[{'route':x['route'],'base':x['base_statuses'],'tip':x['tip_statuses']} for x in rows]},indent=2))
