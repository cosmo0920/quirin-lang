#!/usr/bin/env python3
"""Compare combined results to archived W48 custody and completed RK projections."""
import collections,importlib.util,json,sys
from pathlib import Path
repo,measure,out=map(lambda x:Path(x).resolve(),sys.argv[1:4]);out.mkdir(exist_ok=False)
spec=importlib.util.spec_from_file_location('w44compare',repo/'docs/reports/w44-cross-unit-summary/scripts/compare.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
rk=Path('/tmp/qcore-rk-interceptor-candidates/private/rk-interceptor-candidates/measure-tip-final-v1')
result={}
for route in ['corpus','system','slice']:
 old=module.rows(repo/f'private/w48-batch2-rk-integration/reused-w48/{route}-resource-diagnostics.json')
 new=module.rows(measure/route/'resource-diagnostics.json')
 assert old.keys()==new.keys(),f'{route}: changed denominator'
 changed=[];statuses=collections.Counter()
 for key in old:
  a,b=old[key],new[key];statuses[b['status']]+=1
  if a['status']!=b['status'] or a.get('diagnostics',[])!=b.get('diagnostics',[]):
   changed.append({'key':key,'before':a,'after':b})
 assert not changed,f'{route}: unexpected static custody change against W48'
 before=json.loads((rk/f'{route}-outer.json').read_text());after=json.loads((measure/f'{route}-outer.json').read_text())
 assert before['scenarios'].keys()==after['scenarios'].keys(),f'{route}: scenario set changed'
 scenarios={}
 for name in before['scenarios']:
  a,b=before['scenarios'][name],after['scenarios'][name]
  def identity(row): return tuple(row[k] for k in ['part','library','file','function','occurrence'])
  aa={identity(row):row for row in a['moved']};bb={identity(row):row for row in b['moved']}
  assert len(aa)==len(a['moved']) and len(bb)==len(b['moved'])
  assert aa==bb,f'{route}/{name}: moved runtime rows changed'
  scenarios[name]={'before':{p:a[p] for p in ['custody','null','combined']},'after':{p:b[p] for p in ['custody','null','combined']},'moved_rows_identical':True,'moved_count':len(aa)}
 arows=[json.loads(x) for x in (rk/f'{route}-null.jsonl').read_text().splitlines()]
 brows=[json.loads(x) for x in (measure/f'{route}-null.jsonl').read_text().splitlines()]
 assert len(arows)==len(brows),f'{route}: null denominator changed'
 null_changed=[];custody_changed=[];combined_changed=[]
 for a,b in zip(arows,brows):
  assert all(a.get(k)==b.get(k) for k in ['library','file','function']),f'{route}: null row alignment changed'
  if a['null']!=b['null']: null_changed.append({'before':a,'after':b})
  if a['custody']!=b['custody']: custody_changed.append({'before':a,'after':b})
  if a['combined']!=b['combined']: combined_changed.append({'before':a,'after':b})
 assert not null_changed,f'{route}: null result changed'
 result[route]={'w48_custody_rows':len(new),'w48_status_and_full_diagnostics_identical':True,'static_custody_counts':dict(statuses),'rk_null_rows':len(brows),'null_property_rows_identical':True,'custody_row_changes':custody_changed,'combined_row_changes':combined_changed,'runtime_scenarios':scenarios}
 (out/f'{route}.json').write_text(json.dumps(result[route],indent=2)+'\n')
 print(route,'custody',dict(statuses),'null rows',len(brows),'runtime moves unchanged',len(aa),flush=True)
(out/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
