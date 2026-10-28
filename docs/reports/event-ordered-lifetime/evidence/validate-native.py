"""Compare every native payload; permit only exact, independently checked provenance renewal.
Usage after archival: python3 validate-native.py --receipt native-receipt.json
Raw development mode: --native DIR --base SHA --tip SHA [--output FILE]
"""
import argparse,collections,copy,gzip,hashlib,json,re
from pathlib import Path
if not __debug__:raise SystemExit("Evidence checks require Python without optimization")
import json, hashlib
def ordered(value,keys):
 assert set(value)==set(keys),(set(value),keys)
 return {k:value[k] for k in keys}

def receipt_id(r):
 o=ordered(r['observation'],'schema function declaration route primary_source_sha256 token_stream_sha256 environment_sha256 statement call outer_argument siblings contract checked_predicates prerequisite_ids model_ids'.split())
 for key in ('statement','call','outer_argument'):
  if o[key] is not None:o[key]=ordered(o[key],['start','end'])
 for i,child in enumerate(o['siblings']):
  c=ordered(child,'function argument call outer_argument dereferenced'.split())
  c['call']=ordered(c['call'],['start','end']);c['outer_argument']=ordered(c['outer_argument'],['start','end']);o['siblings'][i]=c
 o['contract']=ordered(o['contract'],['parameters','result']);o['checked_predicates']=dict(sorted(o['checked_predicates'].items()))
 c=ordered(r['context'],'artifact property semantic_profile scope bounds assumptions source_identity build_identity evidence_version'.split())
 raw=json.dumps([o,c,r['graph_sha256'],dict(sorted(r['atom_sha256'].items()))],ensure_ascii=False,separators=(',',':')).encode()
 return hashlib.sha256(raw).hexdigest()

PROVENANCE=None
REVISION_SIDE={}

def load_json(data,name):
 text=data.decode()
 return [json.loads(s) for s in text.splitlines()] if name.endswith('.jsonl') else json.loads(text)

def environments(value, found):
 if isinstance(value,dict):
  table=value.get('reading_site_environments',{}).get('environments',{})
  for identity,text in table.items():
   assert hashlib.sha256(text.encode()).hexdigest()==identity
   assert identity not in found or found[identity]==text
   found[identity]=text
  for child in value.values():environments(child,found)
 elif isinstance(value,list):
  for child in value:environments(child,found)

CONTEXT=set('artifact property semantic_profile scope bounds assumptions source_identity build_identity evidence_version'.split())
def normalize(value,revision,envs,counts,path=''):
 if isinstance(value,list):
  return [normalize(v,revision,envs,counts,path+'/'+str(i)) for i,v in enumerate(value)]
 if not isinstance(value,dict):return value
 value=dict(value)
 if set(value)==CONTEXT:
  if PROVENANCE:PROVENANCE.context(value,REVISION_SIDE[revision],counts)
  assert value['evidence_version']=='contract-hypothesis-graph.v1:'+revision,(path,value['evidence_version'])
  value['evidence_version']='contract-hypothesis-graph.v1:<checked-revision>'
  counts['checked_hypothesis_contexts']+=1
 if {'observation','context','graph_sha256','atom_sha256','id'}<=value.keys():
  assert receipt_id(copy.deepcopy(value))==value['id'],(path,'binding id')
  identity=value['observation']['environment_sha256']
  counts['bindings_with_published_environment' if identity in envs else 'bindings_without_published_environment']+=1
  value['id']='<independently-recomputed-binding-id>'
  counts['independently_recomputed_reading_site_ids']+=1
 if re.fullmatch(r'/contribution/components/\d+/provenance',path):
  assert value['analyzer_revision']=='qcore-audit@0.1.0:'+revision
  value['analyzer_revision']='qcore-audit@0.1.0:<checked-revision>'
  counts['checked_component_revisions']+=1
 out={k:normalize(v,revision,envs,counts,path+'/'+k) for k,v in value.items()}
 if 'reading_site_evidence' in out:
  def anchor(r):
   o=r['observation'];return json.dumps([o['function'],o['declaration'],o['route'],o['statement'],o['call'],o['outer_argument']],sort_keys=True)
  out['reading_site_evidence'].sort(key=lambda r:(anchor(r),json.dumps(r,sort_keys=True,separators=(',',':'))))
 return out

def differences(a,b,path=''):
 if a==b:return []
 if isinstance(a,dict) and isinstance(b,dict):
  out=[]
  for k in sorted(a.keys()|b.keys()):
   at=path+'/'+k.replace('~','~0').replace('/','~1')
   if k not in a or k not in b:out.append({'path':at,'before_present':k in a,'after_present':k in b,'before':a.get(k),'after':b.get(k)})
   else:out.extend(differences(a[k],b[k],at))
  return out
 if isinstance(a,list) and isinstance(b,list) and len(a)==len(b):
  return [p for i,(x,y) in enumerate(zip(a,b)) for p in differences(x,y,path+'/'+str(i))]
 return [{'path':path,'before_present':True,'after_present':True,'before':a,'after':b}]

def main():
 parser=argparse.ArgumentParser();parser.add_argument('--receipt',type=Path);parser.add_argument('--native',type=Path);parser.add_argument('--base');parser.add_argument('--tip');parser.add_argument('--output',type=Path);parser.add_argument('--reviewed-deltas',type=Path);parser.add_argument('--provenance',type=Path);parser.add_argument('--task-preimages',type=Path);args=parser.parse_args()
 if args.receipt:
  receipt=json.loads(args.receipt.read_text());base=receipt['base'];tip=receipt['semantic_tip']
  repo=next(p for p in args.receipt.resolve().parents if (p/'Cargo.toml').exists())
  archives={f['source']:f for f in receipt['files']}
  def read(path):
   entry=archives[str(path)];raw=gzip.decompress((repo/entry['archive']).read_bytes())
   assert len(raw)==entry['bytes'] and hashlib.sha256(raw).hexdigest()==entry['sha256']
   return raw
  cp=next(s for s in archives if s.endswith('/native-comparison.json'))
  comparison=json.loads(read(cp));assert comparison['status']=='complete'
 else:
  assert args.native and args.base and args.tip
  base=args.base;tip=args.tip;read=lambda path:Path(path).read_bytes()
  comparison=json.loads((args.native/'native-comparison.json').read_text());assert comparison['status']=='complete'
 global PROVENANCE,REVISION_SIDE
 REVISION_SIDE={base:0,tip:1}
 if args.receipt:
  def proof_file(suffix):return json.loads(read(next(s for s in archives if s.endswith('/'+suffix))))
  provenance=proof_file('provenance-preimages.json');tasks=proof_file('provenance-task-preimages.json')
 elif args.provenance and args.task_preimages:
  provenance=json.loads(args.provenance.read_text());tasks=json.loads(args.task_preimages.read_text())
 else:provenance=tasks=None
 if provenance is not None:
  from provenance_support import Provenance
  PROVENANCE=Provenance(provenance,tasks)
 result={'schema':'event-ordered-native-attribution/2','reviewed':False,'base':base,'tip':tip,'normalization':'Exact expected analyzer revisions and hypothesis evidence versions; independently recomputed reading-site binding IDs; and cwd-dependent build/configuration/import/task/obligation identities checked from retained preimages. The same raw sources/options/Clang description with only the recorded corpus working-directory change must reproduce both identity chains. Reading receipts are aligned by exact source anchors after their ID checks. Every remaining delta is retained with complete before/after values, including graphs, atoms, contracts, source effects, runtime, auxiliary lifetime summaries, diagnostics and NULL carriers. Published environment strings are independently SHA256 checked; missing exact environment tables are disclosed and do not acquire replay authority. No semantic field or carrier is suppressed.','routes':{}}
 for route,detail in comparison['comparisons'][0]['routes'].items():
  assert detail['figures']['before']==detail['figures']['after'],(route,'figures moved')
  assert not detail['moves'] and not detail['unit_moves']
  assert not detail['artifacts']['only_before'] and not detail['artifacts']['only_after']
  names=sorted({f['file'] for k in ['identical','differing'] for f in detail['artifacts'][k]})
  before={n:read(Path(detail['before_directory'])/n) for n in names}
  after={n:read(Path(detail['after_directory'])/n) for n in names}
  parsed=[{n:load_json(data,n) for n,data in files.items()} for files in [before,after]]
  envsets=[]
  for side in parsed:
   envs={}
   for obj in side.values():environments(obj,envs)
   envsets.append(envs)
  assert envsets[0]==envsets[1],(route,'environment payload change')
  files=[]
  for name in names:
   counts=[collections.Counter(),collections.Counter()]
   inputs=[parsed[0][name],parsed[1][name]]
   if PROVENANCE and name.endswith('.source-effects.json'):
    inputs=[PROVENANCE.effects(inputs[i],i,counts[i]) for i in range(2)]
   a=normalize(inputs[0],base,envsets[0],counts[0]);b=normalize(inputs[1],tip,envsets[1],counts[1])
   deltas=differences(a,b)
   if name==route+'-null.jsonl' or name.endswith('-outer.json') or name.endswith('-null.summary.json') or name.endswith('.chunk-contracts.json') or name.endswith('.source-effects.json'):assert not deltas,(route,name,'carrier changed',deltas[:2])
   assert not any('/reading_site_evidence' in d['path'] or '/null_premises' in d['path'] or '/graph_sha256' in d['path'] or '/atom_sha256' in d['path'] or '/binding_manifest_sha256' in d['path'] for d in deltas),(route,name,'proof binding changed')
   files.append({'file':name,'byte_identical':before[name]==after[name],'before_sha256':hashlib.sha256(before[name]).hexdigest(),'after_sha256':hashlib.sha256(after[name]).hexdigest(),'equal_after_checked_provenance_renewal':not deltas,'before_counts':dict(counts[0]),'after_counts':dict(counts[1]),'deltas':deltas})
  null=next(n for n in names if n==route+'-null.jsonl')
  result['routes'][route]={'figures':detail['figures']['after'],'all_payloads_compared_in_full':True,'null_carriers_equal_after_checked_provenance_renewal':True,'null_carrier_rows':len(parsed[0][null]),'environment_strings_checked':len(envsets[0]),'files':files}
 actual_deltas={r:{f['file']:f['deltas'] for f in d['files'] if f['deltas']} for r,d in result['routes'].items()}
 if args.reviewed_deltas:
  reviewed=json.loads(gzip.decompress(args.reviewed_deltas.read_bytes()) if args.reviewed_deltas.suffix=='.gz' else args.reviewed_deltas.read_bytes())
  assert reviewed['deltas']==actual_deltas,'A semantic delta differs from the explicitly reviewed complete patch ledger'
  result['reviewed']=True;result['review_rationale']=reviewed['rationale']
 if args.output:args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps({r:{'files':len(d['files']),'null_carriers':d['null_carrier_rows'],'environments':d['environment_strings_checked'],'deltas':sum(len(f['deltas']) for f in d['files'])} for r,d in result['routes'].items()}))
if __name__=='__main__':main()
