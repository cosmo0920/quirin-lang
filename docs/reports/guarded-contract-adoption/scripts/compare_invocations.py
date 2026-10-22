#!/usr/bin/env python3
"""Retain complete phase observations; compare the remaining exact provenance."""
import argparse,copy,hashlib,json,subprocess
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
  checked=copy.deepcopy(raw)
  # These exact generated command/output coordinates are proved immediately
  # above, not inferred by deleting arbitrary string-valued differences.
  checked['argv']=['EXACT_ACTUAL_WRAPPER_ARGV_CHECKED']
  checked['reports']['report']='EXACT_ACTUAL_OUTPUT_PATH_CHECKED'
  def paths(value,path=()):
   if isinstance(value,str) and v['inputs_snapshot'] in value:
    assert value.startswith(v['inputs_snapshot']+'/') and path[-1] in {'profile_directory','manifest','path'}
    f=Path(value);assert f.is_file() or path[-1]=='profile_directory' and f.is_dir()
    if f.is_file():transport.append({'side':side,'path':path,'file':str(f),'sha256':sha(f)})
    return '$EXACT_INPUTS'+value[len(v['inputs_snapshot']):]
   if isinstance(value,list):return [paths(x,path+(i,)) for i,x in enumerate(value)]
   if isinstance(value,dict):return {k:paths(x,path+(k,)) for k,x in value.items()}
   return value
  # Complete original observations remain in observed and the retained files.
  # Equality below is explicitly a non-scheduling provenance projection only.
  checked['jobs']=None;checked['memory_admission']=None
  values[side]=paths(checked)
 assert values['base']==values['tip'],'unexplained non-scheduling invocation field'
 producer={}
 for file in ['crates/qcore-audit/src/current_cli.rs','crates/qcore-audit/src/parallel.rs']:
  b=subprocess.check_output(['git','show',inv['base']['analyzer_source']+':'+file]);t=subprocess.check_output(['git','show',inv['tip']['analyzer_source']+':'+file]);assert b==t;producer[file]=hashlib.sha256(b).hexdigest()
 result={'route':a.route,'complete_actual_observations':observed,'all_phase_records_retained':True,'omitted_phase_records':0,'non_scheduling_provenance_projection_equal':True,'observation_equality_claim':False,'exact_path_transports':transport,'unchanged_producer_source_sha256':producer,'scope':'Phase memory/headroom/admitted-job values are actual observations, preserved without normalization or equal-value claim; they convey no verdict authority.'}
a.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'route':a.route,'actual_phase_observations_retained':True}))
