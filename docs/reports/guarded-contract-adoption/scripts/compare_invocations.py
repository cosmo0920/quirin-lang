#!/usr/bin/env python3
"""Retain complete phase observations; compare the remaining exact provenance."""
import argparse,hashlib,json,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('route');p.add_argument('base',type=Path);p.add_argument('tip',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
inv={side:json.loads((path/'invocation.json').read_bytes()) for side,path in [('base',a.base),('tip',a.tip)]}
if a.route=='corpus':
 assert all('actual_runtime_invocation_unavailable' in v for v in inv.values())
 result={'route':a.route,'actual_requested_jobs':{s:v['jobs'] for s,v in inv.items()},'phase_observations':'unavailable: historical pinned route emits no sidecar; not zero','input_invocation_sha256':{s:sha(d/'invocation.json') for s,d in [('base',a.base),('tip',a.tip)]}}
else:
 values={};observed={};transport=[]
 for side,path in [('base',a.base),('tip',a.tip)]:
  v=inv[side];held=v['actual_runtime_invocation'];actual=path/'resource-diagnostics.invocation.json'
  assert sha(actual)==held['sha256'];raw=json.loads(actual.read_bytes());assert raw==held['value']
  assert raw['argv']==['quirin-audit','current']+v['command'][2:]
  assert raw['cwd']==v['cwd'];assert raw['checkout']==v['source_root'];assert raw['source_revision']==v['source_head']
  assert raw['compile_database_sha256']==v['compile_database_sha256'];assert raw['compile_database']==v['compile_database']
  assert raw['profile_directory']==str(Path(v['inputs_snapshot'])/'contract/profiles/fluent-bit')
  assert raw['reports']['report']==str(Path(held['path']).with_name('resource-diagnostics.json'))
  assert raw['jobs']['requested']==v['jobs']==4 and 1<=raw['jobs']['effective']<=4
  phase=raw['memory_admission'];assert phase['omitted']==0
  for r in phase['phases']:
   assert 1<=r['admitted']<=r['requested_maximum']<=4 and r['tasks']>0
   assert (r['observation'] is not None)==(r['observation_status']=='available')
  observed[side]={'invocation_path':held['path'],'invocation_sha256':held['sha256'],'jobs':raw['jobs'],'memory_admission':phase}
  values[side]=raw
 # Compare complete original records recursively. Every changed leaf is
 # retained and classified; no scheduling field is removed or normalized.
 transports=[];scheduling=[];coordinates=[];equal_leaves=0
 def compare(b,t,path=()):
  global equal_leaves
  if b==t:
   equal_leaves+=1;return
  if path==('argv',) or path==('reports','report'):
   coordinates.append({'path':path,'before':b,'after':t,'proof':'each complete value matched its actual held wrapper argv/output immediately above'});return
  if path and path[0]=='jobs':
   assert path==('jobs','effective')
   scheduling.append({'path':path,'before':b,'after':t});return
  if len(path)>=4 and path[:2]==('memory_admission','phases'):
   assert path[3] in {'admitted','observation','observation_status'}
   scheduling.append({'path':path,'before':b,'after':t});return
  if isinstance(b,str) and isinstance(t,str):
   br=inv['base']['inputs_snapshot'];tr=inv['tip']['inputs_snapshot']
   if b.startswith(br+'/') and t.startswith(tr+'/'):
    assert b[len(br):]==t[len(tr):] and path[-1] in {'profile_directory','manifest','path'}
    bf=Path(b);tf=Path(t)
    if path[-1]=='profile_directory':assert bf.is_dir() and tf.is_dir()
    else:assert bf.read_bytes()==tf.read_bytes()
    transports.append({'path':path,'before':b,'after':t,'sha256':sha(bf) if bf.is_file() else None});return
  if isinstance(b,dict) and isinstance(t,dict):
   assert b.keys()==t.keys(),('invocation field domain',path)
   for k in b:compare(b[k],t[k],path+(k,))
   return
  if isinstance(b,list) and isinstance(t,list):
   assert len(b)==len(t),('invocation record count',path)
   for i,(x,y) in enumerate(zip(b,t)):compare(x,y,path+(i,))
   return
  raise AssertionError(('unexplained complete invocation difference',path,b,t))
 compare(values['base'],values['tip'])
 producer={}
 for file in ['crates/qcore-audit/src/current_cli.rs','crates/qcore-audit/src/parallel.rs']:
  b=subprocess.check_output(['git','show',inv['base']['analyzer_source']+':'+file]);t=subprocess.check_output(['git','show',inv['tip']['analyzer_source']+':'+file]);assert b==t;producer[file]=hashlib.sha256(b).hexdigest()
 result={'route':a.route,'complete_actual_observations':observed,'all_phase_records_retained':True,'omitted_phase_records':0,'non_scheduling_provenance_projection_equal':True,'observation_equality_claim':False,'exact_path_transports':transports,'exact_command_output_coordinates':coordinates,'all_scheduling_differences':scheduling,'equal_nodes':equal_leaves,'fields_deleted':0,'unchanged_producer_source_sha256':producer,'scope':'Phase memory/headroom/admitted-job values are actual observations, preserved without normalization or equal-value claim; they convey no verdict authority.'}
a.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'route':a.route,'actual_phase_observations_retained':True}))
