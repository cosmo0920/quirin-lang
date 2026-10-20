from pathlib import Path
from collections import Counter
import hashlib,json
repo=Path.cwd();root=repo/'private/w48-bottom-up-batch4';out=repo/'docs/reports/w48-bottom-up-batch4'
sources={'system':Path('/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source'),'slice':Path('/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source')}
frames={'system':[('src/flb_input_chunk.c','update_logs_tag_records_metrics','in',141,145),('src/flb_task.c','record_unmatched_route_drop_metrics','ins',90,98)],'slice':[('src/flb_input.c','flb_input_instance_init','config',1574,1575)]}
source_hashes={}
for route in ['system','slice']:
 source_hashes[route]={}
 for side in ['before','after']:
  raw=json.loads((root/(route+'-'+side+'-final')/'resource-diagnostics.json').read_text())
  for unit in raw['units']:
   source_hashes[route].setdefault(unit['file'],set()).add(unit['compile_configuration']['raw_source_sha256'])
def observation(route,name,line):
 p=sources[route]/name;lines=p.read_text().splitlines();assert source_hashes[route][name]=={hashlib.sha256(p.read_bytes()).hexdigest()};return {'source_path':name,'line':line,'file_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'excerpt':'\n'.join(f'{i+1}: {lines[i]}' for i in range(max(0,line-3),min(len(lines),line+4)))}
result={}
for route in ['system','slice']:
 pairs=json.loads((root/(route+'-early-rows.json')).read_text());archive=out/'artifacts'/route/'full-row-differences.json.gz'
 if archive.exists():assert pairs==json.loads(__import__('gzip').decompress(archive.read_bytes()))
 removed=Counter();added=Counter();changed=[];added_origins=set();removed_aliases=[]
 for row in pairs:
  a,b=row['before'],row['after'];assert a['status']==b['status'];ca,cb=[Counter(json.dumps(d,sort_keys=True) for d in q.get('diagnostics',[])) for q in [a,b]]
  if ca!=cb:changed.append(row['key'])
  for c,target in [(ca-cb,removed),(cb-ca,added)]:
   for raw,n in c.items():
    d=json.loads(raw);target[(d['category'],d['description'])]+=n
    if target is added:added_origins.add(d['origin'])
    elif d['category']=='unknown_alias':removed_aliases.append({'key':row['key'],'diagnostic':d})
 expected={(name,func,'unresolved resource identity `'+base+'`',name+':'+str(read)) for name,func,base,guard,read in frames[route]}
 assert {(z['key'][1],z['key'][2],z['diagnostic']['description'],z['diagnostic']['origin']) for z in removed_aliases}==expected
 assert all(category=='unknown_alias' for category,desc in added)
 frame_records=[]
 for name,func,base,guard,read in frames[route]:
  frame_records.append({'function':func,'base':base,'guard':observation(route,name,guard),'later_read':observation(route,name,read),'reading':'The old unsupported compound guard conservatively poisoned the base binding. Retaining its complete checked-read plan removes that unsupported effect and its downstream unknown-binding consequence. The original read obligation stays; no scalar/null fact or callback effect is supplied.'})
 result[route]={'changed_diagnostic_rows':len(changed),'all_custody_statuses_identical':True,'changed_keys':changed,'removed_families':[{'category':k[0],'description':k[1],'count':v} for k,v in removed.items()],'added_families':[{'category':k[0],'description':k[1],'count':v} for k,v in added.items()],'removed_aliases':removed_aliases,'removed_alias_frames':frame_records,'new_alias_read_observations':[observation(route,*origin.rsplit(':',1)[:1],int(origin.rsplit(':',1)[1])) for origin in sorted(added_origins)],'new_alias_reading':'Each added UnknownAlias comes from an evaluated, now-retained read of an unresolved chunk/ic/task/instance/file base. These obligations keep the rows Unknown; unsupported container traversal, earlier opaque effects and missing identities are not neutralized.'}
result['mechanism']={'meaning':'Source review explains observed diagnostic changes, not a C extraction/refinement theorem. Full raw rows and complete field comparisons remain separate evidence.','rust_sources':{p:hashlib.sha256((repo/p).read_bytes()).hexdigest() for p in ['crates/qcore-c-gateway/src/legacy_condition_desugar.rs','crates/qcore-c-gateway/src/legacy_cfg_builder.rs','crates/qcore-c-gateway/src/legacy_transfer.rs']},'locations':['legacy_cfg_builder.rs:compile_planned_if route precedes unsupported branch fallback','legacy_transfer.rs:Unknown transfer emits Unsupported and sets potentially written pointer bindings unknown; resource statuses are retained']}
(out/'configured-source-review.json').write_text(json.dumps(result,indent=2)+'\n');print({k:v['changed_diagnostic_rows'] for k,v in result.items() if k!='mechanism'})
