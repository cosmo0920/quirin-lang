#!/usr/bin/env python3
"""Retain complete fresh native artifacts and all raw differences privately.

No identity, graph, canonical source string, body standing or dependency is
removed or normalized. Identity attribution is a separate producer/check.
"""
import argparse, collections, gzip, hashlib, json, runpy, subprocess, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
DIFF=runpy.run_path(str(ROOT/'docs/reports/contract-hypothesis-graph-v1/scripts/compare.py'))['differences']
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('route',choices=['system','slice','corpus']);p.add_argument('base',type=Path);p.add_argument('tip',type=Path);p.add_argument('out',type=Path)
a=p.parse_args();a.out=a.out.resolve();assert '/private/' in str(a.out),'full raw payloads stay private';a.out.mkdir(parents=True,exist_ok=False)
sha=lambda b:hashlib.sha256(b).hexdigest()
names={'diagnostics':'resource-diagnostics.json','contracts':'resource-diagnostics.contracts.json','chunk-contracts':'resource-diagnostics.chunk-contracts.json','source-effects':'resource-diagnostics.source-effects.json','source':'monkey-source.json','null':a.route+'-null.jsonl','null-summary':a.route+'-null.summary.json','outer':a.route+'-outer.json'}
docs={};manifest={};invocations={}
for side,directory in [('base',a.base.resolve()),('tip',a.tip.resolve())]:
 invocation=json.loads((directory/'invocation.json').read_bytes());invocations[side]=invocation
 assert invocation['exit']==0 and 'identity_error' not in invocation
 assert invocation['binary_sha256']==invocation['binary_sha256_after']
 for key in ['source_identity','runner_sha256','profile_sha256','link_map_sha256','report_driver_sha256']+([] if a.route=='corpus' else ['compile_database_sha256','selection_sha256']):
  assert invocation[key]==invocation[key+'_after'],(side,key)
 dest=a.out/side;dest.mkdir();(dest/'invocation.json').write_bytes((directory/'invocation.json').read_bytes());values={};manifest[side]={}
 for meta in ['run.json','completion.json','resource-diagnostics.invocation.json']:
  meta_paths=[path for path in [directory/meta,directory/'corpus'/meta] if path.is_file()]
  assert len(meta_paths)<=1,(side,meta,'ambiguous provenance')
  if meta_paths:
   raw=meta_paths[0].read_bytes();(dest/meta).write_bytes(raw);manifest[side][meta]=dict(source=str(meta_paths[0]),bytes=len(raw),sha256=sha(raw))
   if meta=='run.json' and a.route=='corpus':
    actual=json.loads(raw);assert actual['exit']==0 and actual['binary_sha256']==invocation['binary_sha256']
  elif a.route=='corpus' and meta=='run.json':raise AssertionError('pinned actual-wrapper run.json missing')

 for label,name in names.items():
  candidates=[directory/name,directory/'corpus'/name]
  paths=[path for path in candidates if path.is_file()]
  if not paths:
   assert label in {'source','chunk-contracts','source-effects','null-summary'},(side,name)
   continue
  assert len(paths)==1,(side,name,'ambiguous artifact')
  path=paths[0];raw=path.read_bytes();(dest/(name+'.gz')).write_bytes(gzip.compress(raw,mtime=0))
  manifest[side][label]=dict(source=str(path),bytes=len(raw),sha256=sha(raw))
  values[label]=[json.loads(line) for line in raw.splitlines() if line] if label=='null' else json.loads(raw)
 docs[side]=values
assert docs['base'].keys()==docs['tip'].keys()
for key in ['source_identity','link_map_sha256','profile_sha256','runner_sha256']:
 assert invocations['base'][key]==invocations['tip'][key],key
raw={label:DIFF(docs['base'][label],docs['tip'][label]) for label in docs['base']}
encoded=(json.dumps(raw,indent=2)+'\n').encode();rawpath=a.out/'raw-differences.json.gz';rawpath.write_bytes(gzip.compress(encoded,mtime=0))
def rows(document):
 out={};occurrences=collections.Counter()
 for unit in document['units']:
  for row in unit['functions']:
   key=(unit['library'],unit['file'],row['function']);n=occurrences[key];occurrences[key]+=1;out[key+(n,)]=row
 return out
left,right=[rows(docs[side]['diagnostics']) for side in ['base','tip']];assert left.keys()==right.keys()
moved=[dict(key=key,before=left[key],after=right[key]) for key in left if left[key]['status']!=right[key]['status'] or left[key]['diagnostics']!=right[key]['diagnostics']]
(a.out/'full-semantic-row-differences.json').write_text(json.dumps(moved,indent=2)+'\n')
null_left,null_right=[docs[side]['null'] for side in ['base','tip']];assert len(null_left)==len(null_right)
assert [(r['library'],r['file'],r['function']) for r in null_left]==[(r['library'],r['file'],r['function']) for r in null_right]
summary=dict(route=a.route,rows=len(left),before=dict(collections.Counter(r['status'] for r in left.values())),after=dict(collections.Counter(r['status'] for r in right.values())),semantic_rows_changed=len(moved),safe_gains=[m['key'] for m in moved if m['before']['status']!='Safe' and m['after']['status']=='Safe'],safe_regressions=[m['key'] for m in moved if m['before']['status']=='Safe' and m['after']['status']!='Safe'],null_rows=len(null_left),full_null_dictionaries_equal=all(x['null']==y['null'] for x,y in zip(null_left,null_right)),all_raw_fields_retained=True,raw_leaf_counts={key:len(value) for key,value in raw.items()},raw_differences=dict(path=str(rawpath),sha256=sha(rawpath.read_bytes()),bytes=rawpath.stat().st_size),identity_attribution='Required separately; no raw delta is hidden by this collector.',unsafe_qualification='All published Unsafe labels remain provisional, not upstream defect claims.')
(a.out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');(a.out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
