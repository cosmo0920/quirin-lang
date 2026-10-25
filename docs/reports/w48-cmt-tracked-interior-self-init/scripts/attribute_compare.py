"""Retain raw payloads; reconstruct only exact proven metadata; expose remainder."""
from pathlib import Path
import json,gzip,runpy,copy,collections,sys,hashlib
p=Path('private/w48-cmt-tracked-interior');route=sys.argv[1];out=p/'comparisons'/route;proofdir=p/'replay-v3'
read=lambda f:json.loads(Path(f).read_bytes());unpack=lambda f:json.loads(gzip.decompress(Path(f).read_bytes()));write=lambda f,x:Path(f).write_bytes(gzip.compress((json.dumps(x,indent=2)+'\n').encode(),mtime=0))
diff=runpy.run_path('docs/reports/contract-hypothesis-graph-v1/scripts/compare.py')['differences'];manifest=read(out/'manifest.json')
data={side:{n:unpack(out/side/(n+'.gz')) if n.endswith('.json') else [json.loads(l) for l in gzip.decompress((out/side/(n+'.gz')).read_bytes()).splitlines() if l] for n in manifest[side] if n.endswith(('.json','.jsonl'))} for side in ['base','tip']}
raw=unpack(out/'raw-differences.json.gz');left,right=[copy.deepcopy(data[s]) for s in ['base','tip']]
old='contract-hypothesis-graph.v1:'+read(p/'config.json')['base'];new='contract-hypothesis-graph.v1:'+read(p/'semantic-freeze.json')['head'];CONTEXT={'artifact','property','semantic_profile','scope','bounds','assumptions','source_identity','build_identity','evidence_version'}
def original_versions(v,expected):
 if isinstance(v,list):
  for x in v:original_versions(x,expected)
 elif isinstance(v,dict):
  if set(v)==CONTEXT:assert v['evidence_version']==expected
  for x in v.values():original_versions(x,expected)
for side,version in [('base',old),('tip',new)]:original_versions(data[side],version)
proof=read(proofdir/(route+'-readings-proof.json'));assert proof['source_binding']=='Open' and proof['truth_transport'] is False
for name in ['resource-diagnostics.json','resource-diagnostics.contracts.json']:
 for side in ['base','tip']:
  match=[(k,v) for k,v in proof['input_sha256'].items() if k.endswith('/'+('base-'+route if side=='base' else 'tip-'+route)+'/'+name)];assert len(match)==1 and match[0][1]==manifest[side][name]['sha256']
records={}
for r in proof['records']:
 key=r['tip_id'];assert r['actual_bindings_reproduced'] and r['exact_observation_unchanged'] and r['exact_environment_unchanged']
 if key in records:assert records[key]==r
 records[key]=r
restored=0
for unit in right['resource-diagnostics.json']['units']:
 for row in unit['functions']:
  if 'reading_site_evidence' not in row:continue
  rs=[]
  for receipt in row['reading_site_evidence']:
   record=records[receipt['id']];assert record['tip_receipt']==receipt;rs.append(record['base_receipt']);restored+=1
  row['reading_site_evidence']=sorted(rs,key=lambda r:r['id'])
assert restored==len(proof['records'])
version_paths=[]
def versions(v,path=()):
 if isinstance(v,list):return [versions(x,path+(i,)) for i,x in enumerate(v)]
 if not isinstance(v,dict):return v
 r={k:versions(x,path+(k,)) for k,x in v.items()}
 if set(r)==CONTEXT:
  assert r['evidence_version'] in [old,new]
  if r['evidence_version']==new:r['evidence_version']=old;version_paths.append(path+('evidence_version',))
 return r
right=versions(right)
time=read(proofdir/'time'/route/'time-identity-review.json');time_records=[]
if time.get('identity_remap_needed') is not False:
 assert time['source_algorithms_equal_semantic_base'] and time['wrong_time_negative_control']['rejected'];a,b=time['before_key'],time['after_key']
 for path in time['paths']:
  x,y=left['resource-diagnostics.contracts.json'],right['resource-diagnostics.contracts.json']
  for k in path:x=x[k];y=y[k]
  assert a in x and b in y and x[a]==y[b]=={} and a not in y
  assert all(x[k]==y[k] for k in x.keys()&y.keys());y[a]=y.pop(b);assert x==y;time_records.append(path)
metadata_files={'run.json','producer-invocation.json','invocation.json','resource-diagnostics.invocation.json'}
remaining={n:diff(left[n],right[n]) for n in left if n not in metadata_files};write(out/'remaining-differences.json.gz',remaining)
write(out/'exact-version-attribution.json.gz',version_paths);write(out/'measurement-provenance-differences.json.gz',{n:raw[n] for n in raw if n in metadata_files})
result={'route':route,'original_raw_difference_counts':{k:len(v) for k,v in raw.items()},'remaining_difference_counts':{k:len(v) for k,v in remaining.items()},'complete_reading_receipts_replayed':restored,'complete_canonical_captures_validated':proof['canonical_captures_validated'],'evidence_version_occurrences_reconstructed':len(version_paths),'empty_summary_time_remap_paths':time_records,'measurement_metadata_files_separately_retained':sorted(metadata_files&raw.keys()),'fields_erased':0,'unexplained_semantic_differences':'pending explicit native NULL and projection accounting','source_binding':'Open','truth_transport':False}
(out/'attributed-comparison.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
