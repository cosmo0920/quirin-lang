#!/usr/bin/env python3
"""Archive complete paired artifacts, erase only added site records, compare every old field."""
import argparse,collections,copy,gzip,hashlib,importlib.util,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
spec=importlib.util.spec_from_file_location('tree_diff',ROOT/'docs/reports/contract-hypothesis-graph-v1/scripts/compare.py');old=importlib.util.module_from_spec(spec);spec.loader.exec_module(old)
CONTEXT={'artifact','property','semantic_profile','scope','bounds','assumptions','source_identity','build_identity','evidence_version'}
def write(path,value):path.write_bytes(gzip.compress((json.dumps(value,indent=2)+'\n').encode(),mtime=0))
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('route');p.add_argument('before',type=Path);p.add_argument('after',type=Path);p.add_argument('out',type=Path);a=p.parse_args();a.out.mkdir(parents=True,exist_ok=False)
 names={'diagnostics':'resource-diagnostics.json','contracts':'resource-diagnostics.contracts.json','chunk-contracts':'resource-diagnostics.chunk-contracts.json','source-effects':'resource-diagnostics.source-effects.json','source':'monkey-source.json','null':a.route+'-null.jsonl','null-summary':a.route+'-null.summary.json','outer':a.route+'-outer.json'}
 data={};invocations={}
 for side,directory in [('before',a.before),('after',a.after)]:
  invocations[side]=json.loads((directory/'invocation.json').read_text());inv=invocations[side];assert inv['exit']==0 and inv['binary_sha256']==inv['binary_sha256_after']
  target=a.out/side;target.mkdir();(target/'invocation.json').write_text(json.dumps(inv,indent=2)+'\n');docs={};manifest={}
  for kind,name in names.items():
   path=directory/name
   if not path.exists():assert kind in {'chunk-contracts','source-effects','source'};continue
   raw=path.read_bytes();(target/(name+'.gz')).write_bytes(gzip.compress(raw,mtime=0));manifest[name]={'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest()}
   docs[kind]=[json.loads(x) for x in raw.splitlines()] if name.endswith('.jsonl') else json.loads(raw)
  (target/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');data[side]=docs
 assert data['before'].keys()==data['after'].keys()
 assert invocations['before']['contract_inputs_root']==invocations['after']['contract_inputs_root']
 assert invocations['before']['process_cwd']==invocations['after']['process_cwd']
 raw_diff={k:old.differences(data['before'][k],data['after'][k]) for k in data['before']};write(a.out/'raw-differences.json.gz',raw_diff)
 left=copy.deepcopy(data['before']);right=copy.deepcopy(data['after']);erased=[];receipts=[]
 for u,unit in enumerate(right['diagnostics']['units']):
  if 'reading_site_domain' in unit:erased.append({'path':['diagnostics','units',u,'reading_site_domain'],'value':unit.pop('reading_site_domain')})
  for f,row in enumerate(unit['functions']):
   if 'reading_site_evidence' in row:
    value=row.pop('reading_site_evidence');assert all(r['coverage']=='selected-observations-only' and r['universal_disposition']=='open' and r['claim']=='accepted-local-checker-fragment' and r['context']==row['hypothesis_context'] for r in value)
    receipts.extend(value);erased.append({'path':['diagnostics','units',u,'functions',f,'reading_site_evidence'],'value':value})
 versions=[]
 def restore_versions(value,path=()):
  if isinstance(value,list):return [restore_versions(v,path+(i,)) for i,v in enumerate(value)]
  if not isinstance(value,dict):return value
  result={k:restore_versions(v,path+(k,)) for k,v in value.items()}
  if CONTEXT<=result.keys():
   expected='contract-hypothesis-graph.v1:'+invocations['after']['analyzer_source_commit']
   assert result['evidence_version']==expected,(path,result['evidence_version'])
   original='contract-hypothesis-graph.v1:'+invocations['before']['analyzer_source_commit']
   versions.append({'path':path+('evidence_version',),'before':original,'after':expected,'attribution':'independently hashed immutable binary source receipts; existing Context constructor build-commit suffix only'})
   result['evidence_version']=original
  return result
 right=restore_versions(right)
 differences={k:old.differences(left[k],right[k]) for k in left};write(a.out/'residual-differences.json.gz',differences);write(a.out/'additive-erasure.json.gz',erased);write(a.out/'exact-version-attribution.json.gz',versions);write(a.out/'production-site-receipts.json.gz',receipts)
 count=lambda docs:dict(collections.Counter(row['status'] for unit in docs['diagnostics']['units'] for row in unit['functions']))
 checks={'identical_input_paths':True,'identical_process_cwd':True,'all_existing_artifacts_equal_after_exact_additive_erasure_and_version_attribution':all(not d for d in differences.values()),'static_counts_equal':count(left)==count(right),'full_null_equal':left['null']==right['null'],'null_summary_equal':left['null-summary']==right['null-summary'],'full_runtime_equal':left['outer']==right['outer']}
 result=dict(route=a.route,checks=checks,before_counts=count(left),after_counts=count(right),null_records=len(left['null']),site_observations=len(receipts),site_observations_by_declaration=dict(collections.Counter(r['observation']['declaration'] for r in receipts)),raw_differences={k:len(v) for k,v in raw_diff.items()},residual_differences={k:len(v) for k,v in differences.items()},version_records=len(versions),all_pass=all(checks.values()))
 (a.out/'comparison.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2));return 0 if result['all_pass'] else 1
if __name__=='__main__':raise SystemExit(main())
