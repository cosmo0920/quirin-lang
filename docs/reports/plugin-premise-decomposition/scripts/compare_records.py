"""Archive every raw field; reverse only separately checked bounded metadata."""
import argparse,collections,copy,gzip,hashlib,json,runpy
from pathlib import Path
ROOT=Path.cwd();REPORT=ROOT/'docs/reports/plugin-premise-decomposition';P=ROOT/'private/plugin-premise-decomposition'
diff=runpy.run_path(str(ROOT/'docs/reports/contract-hypothesis-graph-v1/scripts/compare.py'))['differences'];graph=runpy.run_path(str(REPORT/'scripts/graph_metadata.py'))
p=argparse.ArgumentParser();p.add_argument('route');p.add_argument('base',type=Path);p.add_argument('tip',type=Path);p.add_argument('out',type=Path);p.add_argument('--known',type=Path,required=True);p.add_argument('--reading-proof',type=Path,required=True);a=p.parse_args();a.out.mkdir(parents=True,exist_ok=False)
read=lambda p:json.loads(p.read_bytes());sha=lambda b:hashlib.sha256(b).hexdigest()
def write(path,value):path.write_bytes(gzip.compress((json.dumps(value,indent=2)+'\n').encode(),mtime=0))
names={'diagnostics':'resource-diagnostics.json','contracts':'resource-diagnostics.contracts.json','chunk-contracts':'resource-diagnostics.chunk-contracts.json','source-effects':'resource-diagnostics.source-effects.json','source':'monkey-source.json','null':a.route+'-null.jsonl','null-summary':a.route+'-null.summary.json','outer':a.route+'-outer.json'};data={};invocations={}
for side,directory in [('base',a.base),('tip',a.tip)]:
 invocations[side]=read(directory/'invocation.json');inv=invocations[side];assert inv['exit']==0 and inv['binary_sha256']==inv['binary_sha256_after'];assert inv['inputs_before']==inv['inputs_after'];dest=a.out/side;dest.mkdir();(dest/'invocation.json').write_text(json.dumps(inv,indent=2)+'\n');docs={};manifest={}
 for label,name in names.items():
  path=directory/name
  if not path.exists():assert label in {'source','chunk-contracts','source-effects'};continue
  raw=path.read_bytes();(dest/(name+'.gz')).write_bytes(gzip.compress(raw,mtime=0));manifest[name]={'bytes':len(raw),'sha256':sha(raw)};docs[label]=[json.loads(x) for x in raw.splitlines() if x] if label=='null' else json.loads(raw)
 (dest/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');data[side]=docs
assert data['base'].keys()==data['tip'].keys();raw={key:diff(data['base'][key],data['tip'][key]) for key in data['base']};write(a.out/'raw-differences.json.gz',raw)
known_spec=read(a.known);known=known_spec['exact_strings'];roots=[invocations[side]['inputs_snapshot'] for side in ['base','tip']];roots.sort(key=len,reverse=True);changes=[];snapshot_files={}
def migration(value,path=(),side='tip'):
 if isinstance(value,str):
  before=value
  if side=='tip':value=known.get(value,value)
  for root in roots:
   if root not in value:continue
   # Snapshot spelling is admissible only in actual file-valued metadata.
   # Its counterpart is held independently, exists inside the other immutable
   # snapshot, and has identical bytes or an already proven whole-file inverse.
   assert path and path[-1] in {'module','source_module','lock','path','$key'}, ('snapshot path outside file metadata',path)
   assert value.startswith(root) and value.count(root)==1, ('not a single complete file path',path)
   actual=Path(value).resolve();relative=actual.relative_to(Path(root).resolve())
   other_root=next(r for r in roots if r!=root);other=Path(other_root)/relative
   assert actual.is_file() and other.is_file(), ('snapshot metadata must identify two existing held files',path)
   actual_sha=sha(actual.read_bytes());other_sha=sha(other.read_bytes())
   assert actual_sha==other_sha or known.get(actual_sha)==other_sha or known.get(other_sha)==actual_sha, ('snapshot files differ without independent complete-file proof',path)
   file_key=(str(actual),str(other),actual_sha,other_sha)
   if file_key not in snapshot_files:snapshot_files[file_key]={'held_file':str(actual),'counterpart_file':str(other),'relative_file':str(relative),'held_sha256':actual_sha,'counterpart_sha256':other_sha,'proof':'byte-exact identity' if actual_sha==other_sha else 'independently held whole-file inverse from known input catalog','metadata_occurrences':0,'field_kinds':[]}
   snapshot_files[file_key]['metadata_occurrences']+=1
   if path[-1] not in snapshot_files[file_key]['field_kinds']:snapshot_files[file_key]['field_kinds'].append(path[-1])
   value=value.replace(root,'$INPUTS')
  if value!=before:changes.append({'side':side,'path':path,'operation':'independently_proved_exact_string_or_snapshot_root','before_sha256':sha(before.encode()),'after_sha256':sha(value.encode())})
  return value
 if isinstance(value,list):return [migration(x,path+(i,),side) for i,x in enumerate(value)]
 if isinstance(value,dict):
  result={}
  for key,item in value.items():
   new_key=migration(key,path+('$key',),side);assert new_key not in result;result[new_key]=migration(item,path+(key,),side)
  return result
 return value
left=copy.deepcopy(data['base']);right={};graph_receipts=[]
for label,document in data['tip'].items():
 result,hashes,receipts=graph['attribute'](document);right[label]=result;graph_receipts.extend({'artifact':label,**item} for item in receipts)
# Full observations and existing receipt fields are never erased. The Rust
# proof has checked canonical effective BASE preimages and unchanged bind both
# sides, including exact non-environment payload and all Context coordinates.
proof=read(a.reading_proof);assert proof['source_binding']=='open' and proof['truth_transport'] is False
assert proof['input_sha256']['base_diagnostics']==sha((a.base/names['diagnostics']).read_bytes());assert proof['input_sha256']['tip_diagnostics']==sha((a.tip/names['diagnostics']).read_bytes())
base_receipts={}
for unit in data['base']['diagnostics']['units']:
 for row in unit['functions']:
  for receipt in row.get('reading_site_evidence',[]):
   if receipt['id'] in base_receipts:assert base_receipts[receipt['id']]==receipt
   base_receipts[receipt['id']]=receipt
receipt_map={}
for receipt in proof['receipts']:
 if receipt['tip_receipt'] in receipt_map:assert receipt_map[receipt['tip_receipt']]==receipt['base_receipt']
 receipt_map[receipt['tip_receipt']]=receipt['base_receipt']
new_captures=[];restored=0
for ui,unit in enumerate(right['diagnostics']['units']):
 if 'reading_site_environments' in unit:
  value=unit.pop('reading_site_environments');assert set(value)=={'schema','environments','claim','universal_disposition'};assert value['schema']=='libc-reading-site-effective-environments.v1' and value['claim']=='exact-canonical-effective-environment-bytes-only' and value['universal_disposition']=='open';new_captures.append({'unit':ui,'value':value})
 for row in unit['functions']:
  if 'reading_site_evidence' in row:
   row['reading_site_evidence']=[copy.deepcopy(base_receipts[receipt_map[receipt['id']]]) for receipt in row['reading_site_evidence']];row['reading_site_evidence'].sort(key=lambda r:r['id']);restored+=len(row['reading_site_evidence'])
assert restored==len(proof['receipts'])
CONTEXT={'artifact','property','semantic_profile','scope','bounds','assumptions','source_identity','build_identity','evidence_version'};version_changes=[]
def validate_original_versions(v,expected):
 if isinstance(v,list):
  for item in v:validate_original_versions(item,expected)
 elif isinstance(v,dict):
  if set(v)==CONTEXT:assert v['evidence_version']==expected
  for item in v.values():validate_original_versions(item,expected)
for side,docs in data.items():validate_original_versions(docs,'contract-hypothesis-graph.v1:'+invocations[side]['analyzer_source'])

def versions(v,path=()):
 if isinstance(v,list):return [versions(x,path+(i,)) for i,x in enumerate(v)]
 if not isinstance(v,dict):return v
 r={key:versions(x,path+(key,)) for key,x in v.items()}
 if CONTEXT==set(r):
  old='contract-hypothesis-graph.v1:'+invocations['base']['analyzer_source'];new='contract-hypothesis-graph.v1:'+invocations['tip']['analyzer_source'];assert r['evidence_version'] in [old,new]
  if r['evidence_version']==new:version_changes.append({'path':path+('evidence_version',),'before':old,'after':new});r['evidence_version']=old
 return r
right=versions(right)
# The actual contract dependency producer stores complete Value JSON strings
# in a BTreeSet, then parses them to publish Context arrays. Reapply precisely
# that producer ordering after independently verified coordinate restoration.
context_orders=[]
def sorted_contexts(document,check_only=False):
 for declaration,record in document['contracts']['contract_dependencies']['declarations'].items():
  evidence=record.get('hypothesis_evidence',{});contexts=evidence.get('contexts')
  if contexts is None:continue
  assert all(set(c)==CONTEXT for c in contexts)
  key=lambda value:json.dumps(value,sort_keys=True,ensure_ascii=False,separators=(',',':'))
  ordered=sorted(contexts,key=key);assert len(set(map(key,contexts)))==len(contexts)
  if check_only:assert contexts==ordered
  else:
   if contexts!=ordered:context_orders.append(declaration)
   evidence['contexts']=ordered
for document in data.values():sorted_contexts(document,check_only=True)
left=migration(left,side='base');right=migration(right)
sorted_contexts(left,check_only=True);sorted_contexts(right)
(a.out/'context-order-attribution.json').write_text(json.dumps({'producer':'contract_dependencies.rs Facts::absorb Value::to_string BTreeSet ordering','changed_declarations':context_orders,'fields_erased':0},indent=2)+'\n')
(a.out/'snapshot-file-provenance.json').write_text(json.dumps({'file_metadata_fields':['module','source_module','lock','path','$key'],'no_code_or_general_string_normalization':True,'records':list(snapshot_files.values())},indent=2)+'\n')
remaining={key:diff(left[key],right[key]) for key in left};write(a.out/'remaining-differences.json.gz',remaining);write(a.out/'graph-attribution.json.gz',graph_receipts);write(a.out/'exact-string-attribution.json.gz',changes);write(a.out/'exact-version-attribution.json.gz',version_changes);write(a.out/'new-diagnostic-environments.json.gz',new_captures)
counts=lambda docs:dict(collections.Counter(row['status'] for unit in docs['diagnostics']['units'] for row in unit['functions']))
result={'route':a.route,'base':counts(data['base']),'tip':counts(data['tip']),'null_rows':len(left['null']),'full_null_equal':left['null']==right['null'],'full_runtime_equal':left['outer']==right['outer'],'raw_leaf_counts':{key:len(value) for key,value in raw.items()},'remaining_leaf_counts':{key:len(value) for key,value in remaining.items()},'existing_reading_observations_exactly_reconstructed':restored,'all_existing_fields_equal':all(not value for value in remaining.values()),'known_catalog_sha256':sha(a.known.read_bytes()),'reading_proof_sha256':sha(a.reading_proof.read_bytes()),'source_binding':'open'};(a.out/'comparison.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2));raise SystemExit(0 if result['all_existing_fields_equal'] else 1)
