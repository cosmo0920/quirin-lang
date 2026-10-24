from pathlib import Path
import json,collections
p=Path('private/w48-cmt-tracked-interior');names=['metric_prepare_storage','metric_index_add','map_metric_create','map_metric_get_unlocked','cmt_map_create','cmt_map_metric_get'];data={}
for side in ['base','tip']:
 d=json.load(open(p/(side+'-corpus/resource-diagnostics.json')));selected={}
 for u in d['units']:
  if u['library']!='cmetrics':continue
  for row in u['functions']:
   if row['function'] in names:
    assert row['function'] not in selected;selected[row['function']]={'library':u['library'],'file':u['file'],'row':row}
 assert set(selected)==set(names);data[side]=selected
out=[]
for name in names:
 b,t=[data[s][name]['row'] for s in ['base','tip']];c=[collections.Counter(json.dumps(x,sort_keys=True) for x in r['diagnostics']) for r in [b,t]]
 out.append({'function':name,'file':data['base'][name]['file'],'base_status':b['status'],'tip_status':t['status'],'base_diagnostics':len(b['diagnostics']),'tip_diagnostics':len(t['diagnostics']),'removed_diagnostics':[json.loads(x) for x in (c[0]-c[1]).elements()],'added_diagnostics':[json.loads(x) for x in (c[1]-c[0]).elements()],'complete_base_row':b,'complete_tip_row':t})
(p/'production-targets.json').write_text(json.dumps({'targets':out,'getter_closure_gain':int(data['base']['cmt_map_metric_get']['row']['status']!='Safe' and data['tip']['cmt_map_metric_get']['row']['status']=='Safe'),'complete_rows_retained':True},indent=2)+'\n')
print([(x['function'],x['base_status'],x['tip_status'],x['base_diagnostics'],x['tip_diagnostics']) for x in out])
