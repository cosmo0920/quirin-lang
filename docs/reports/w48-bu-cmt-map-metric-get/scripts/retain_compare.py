"""Retain every native payload and raw difference, with no erasure/normalization."""
from pathlib import Path
import collections,gzip,hashlib,json,runpy,sys
root=Path.cwd();p=root/'private/w48-cmt-map';route=sys.argv[1];assert route in ['corpus','system','slice']
out=p/'comparisons'/route;out.mkdir(parents=True,exist_ok=False)
diff=runpy.run_path(str(root/'docs/reports/contract-hypothesis-graph-v1/scripts/compare.py'))['differences'];sha=lambda b:hashlib.sha256(b).hexdigest()
read=lambda path:json.loads(Path(path).read_bytes())
def write(path,value):Path(path).write_bytes(gzip.compress((json.dumps(value,indent=2)+'\n').encode(),mtime=0))
directories={'base':p/('base-'+route+'-v3'),'tip':p/('tip-'+route)}
inv={side:read(p/(side+'-corpus-v3-invocation.json' if side=='base' else 'tip-corpus-invocation.json')) if route=='corpus' else read(directory/'invocation.json') for side,directory in directories.items()}
for side,r in inv.items():assert r['exit']==0 and r['stable'] and r['before']==r['after'],(side,route)
keys=['manifest','inputs','source','database','link_map','namespace_wrapper']+(['monkey_database','package_files'] if route=='corpus' else ['selection','driver'])
for key in keys:assert inv['base']['before'][key]==inv['tip']['before'][key],(route,key)
if route=='corpus':assert inv['base']['before']['inputs']['scripts/run_pinned_audit.py']==inv['tip']['before']['runner']
assert inv['base']['clang']==inv['tip']['clang']
(out/'input-invariants.json').write_text(json.dumps({'route':route,'all_recorded_before_after_inputs_stable':True,'paired_equal_keys':keys,'identical_actual_clang_banner':inv['base']['clang'],'binary_source_separate_from_working_head':True,'all_measurement_invocations_retained':True},indent=2)+'\n')
data={};manifest={}
for side,directory in directories.items():
 dest=out/side;dest.mkdir();manifest[side]={};docs={}
 files={f.name:f for f in directory.iterdir() if f.is_file()}
 files['producer-invocation.json']=p/(side+'-corpus-v3-invocation.json' if side=='base' else 'tip-corpus-invocation.json') if route=='corpus' else directory/'invocation.json'
 if route=='corpus':
  for label,suffix in [('null.jsonl','null.jsonl'),('outer.json','outer.json')]:files[label]=p/(side+'-corpus-v3-'+suffix if side=='base' else 'tip-corpus-'+suffix)
  summaries=list(p.glob(('base-corpus-v3-null' if side=='base' else 'tip-corpus-null')+'*.summary.json'))
  assert len(summaries)<=1
  if summaries:files['null.summary.json']=summaries[0]
 for name,path in files.items():
  raw=path.read_bytes();(dest/(name+'.gz')).write_bytes(gzip.compress(raw,mtime=0));manifest[side][name]={'source':str(path),'sha256':sha(raw),'bytes':len(raw)}
  if name.endswith('.json'):docs[name]=json.loads(raw)
  elif name.endswith('.jsonl'):docs[name]=[json.loads(line) for line in raw.splitlines() if line]
 data[side]=docs
assert data['base'].keys()==data['tip'].keys(),(data['base'].keys(),data['tip'].keys())
raw={name:diff(data['base'][name],data['tip'][name]) for name in data['base']};write(out/'raw-differences.json.gz',raw)
def rows(document):
 result={};n=collections.Counter()
 for unit in document['units']:
  for row in unit['functions']:
   base=(unit['library'],unit['file'],row['function']);key=base+(n[base],);n[base]+=1;assert key not in result;result[key]=row
 return result
left,right=[rows(data[side]['resource-diagnostics.json']) for side in ['base','tip']];assert left.keys()==right.keys()
moves=[]
for key,before in left.items():
 after=right[key]
 if before['status']!=after['status'] or before['diagnostics']!=after['diagnostics'] or before['contract_dependencies']!=after['contract_dependencies']:
  counters=[collections.Counter(json.dumps(d,sort_keys=True) for d in row['diagnostics']) for row in [before,after]]
  moves.append({'key':key,'before':before,'after':after,'removed_diagnostics':[json.loads(x) for x in (counters[0]-counters[1]).elements()],'added_diagnostics':[json.loads(x) for x in (counters[1]-counters[0]).elements()]})
write(out/'full-row-movements.json.gz',moves)
nulls=[data[s]['null.jsonl'] for s in ['base','tip']];assert len(nulls[0])==len(nulls[1]);assert [[(r['library'],r['file'],r['function']) for r in side] for side in nulls][0]==[(r['library'],r['file'],r['function']) for r in nulls[1]]
null_moves=[{'key':(a['library'],a['file'],a['function']),'before':a,'after':b} for a,b in zip(*nulls) if a['null']!=b['null']]
write(out/'full-null-movements.json.gz',null_moves)
summary={'route':route,'resource_rows':len(left),'base_statuses':dict(collections.Counter(r['status'] for r in left.values())),'tip_statuses':dict(collections.Counter(r['status'] for r in right.values())),'resource_semantic_rows_changed':len(moves),'safe_gains':[x['key'] for x in moves if x['before']['status']!='Safe' and x['after']['status']=='Safe'],'safe_losses':[x['key'] for x in moves if x['before']['status']=='Safe' and x['after']['status']!='Safe'],'null_rows':len(nulls[0]),'complete_null_dictionary_changes':len(null_moves),'raw_difference_counts':{k:len(v) for k,v in raw.items()},'all_raw_fields_retained':True,'attribution':'pending; collector does not hide any difference','unsafe_qualification':'all Unsafe labels remain provisional, not upstream defect claims'}
(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
