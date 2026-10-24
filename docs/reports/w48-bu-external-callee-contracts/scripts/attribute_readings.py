"""Replay exact effective environments and typed receipt ID preimages; no C truth transport."""
from pathlib import Path
import json,hashlib,copy,argparse,subprocess,collections
ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--evidence-root',type=Path,default=Path(__file__).parent);ap.add_argument('--repo',type=Path,default=Path.cwd());ap.add_argument('--routes',nargs='+',default=['corpus','system','slice']);a=ap.parse_args();P=a.evidence_root.resolve()
read=lambda f:json.loads(f.read_bytes());serial=lambda v:json.dumps(v,ensure_ascii=False,separators=(',',':')).encode();sha=lambda b:hashlib.sha256(b).hexdigest()
obs_fields='schema function declaration route primary_source_sha256 token_stream_sha256 environment_sha256 statement call outer_argument siblings contract checked_predicates prerequisite_ids model_ids'.split()
ctx_fields='artifact property semantic_profile scope bounds assumptions source_identity build_identity evidence_version'.split()
def ordered(v,fields):
 assert set(v)==set(fields),(set(v),fields)
 return {k:copy.deepcopy(v[k]) for k in fields}
def span(v):return ordered(v,['start','end'])
def receipt_id(r):
 o=ordered(r['observation'],obs_fields)
 for k in ['statement','call']:o[k]=span(o[k])
 if o['outer_argument']is not None:o['outer_argument']=span(o['outer_argument'])
 siblings=[]
 for s in o['siblings']:
  v=ordered(s,['function','argument','call','outer_argument','dereferenced']);v['call']=span(v['call']);v['outer_argument']=span(v['outer_argument']);siblings.append(v)
 o['siblings']=siblings;o['contract']=ordered(o['contract'],['parameters','result']);o['checked_predicates']=dict(sorted(o['checked_predicates'].items()))
 c=ordered(r['context'],ctx_fields)
 return sha(serial([o,c,r['graph_sha256'],dict(sorted(r['atom_sha256'].items()))]))
env_proof=read(P/'environment-attribution.json');root_pairs={r['hashes']['tip']['environment_sha256']:r['hashes']['base']['environment_sha256'] for r in env_proof['records']}
for route in a.routes:
 proof=read(P/(route+'-identity-attribution.json'));known={(r['tip'],f):r['base'] for r in proof['exact_field_scoped_pairs'] for f in r['allowed_fields']}
 dirs={s:P/('base-3'if s=='base'else'tip-1')/route for s in ['base','tip']}
 assert all(read(d/'invocation.json').get('exit')==0 for d in dirs.values())
 docs={s:read(d/('corpus/resource-diagnostics.json'if route=='corpus'else'resource-diagnostics.json')) for s,d in dirs.items()}
 roots={s:read(P/(s+'-environments')/route/(s+'-environment.json')) for s in dirs}
 producers={}
 for f in ['crates/qcore-c-gateway/src/reading_site_evidence.rs','crates/qcore-schemas/src/contract.rs','crates/qcore-schemas/src/hypothesis.rs']:
  raw=[subprocess.check_output(['git','show',proof['commits'][s]+':'+f],cwd=a.repo) for s in ['base','tip']];assert raw[0]==raw[1];producers[f]=sha(raw[0])
 env_records=[];receipt_records=[];total=0
 for ui,(bu,tu) in enumerate(zip(docs['base']['units'],docs['tip']['units'])):
  assert (bu['library'],bu['file'])==(tu['library'],tu['file'])
  bv,tv=[u.get('reading_site_environments') for u in [bu,tu]];assert (bv is None)==(tv is None)
  env_map={}
  if bv is not None:
   assert {k:v for k,v in bv.items()if k!='environments'}=={k:v for k,v in tv.items()if k!='environments'}
   for h,canonical in tv['environments'].items():
    assert sha(canonical.encode())==h;v=json.loads(canonical);assert serial(v).decode()==canonical
    for n in ['writev','thrd_current']:
     assert v['functions'][n]==roots['tip']['functions'][n];del v['functions'][n]
    changed=[]
    for k,old in roots['base']['source_assumptions'].items():
     if old!=roots['tip']['source_assumptions'][k]:assert v['source_assumptions'][k]==roots['tip']['source_assumptions'][k];v['source_assumptions'][k]=old;changed.append(k)
    assert len(changed)==2
    imports=0
    for entry in v.get('cross_unit_import_identities',{}).values():
     hnew=entry['environment_sha256'];assert hnew in root_pairs;entry['environment_sha256']=root_pairs[hnew];imports+=1
    raw=serial(v).decode();bh=sha(raw.encode());assert bv['environments'].get(bh)==raw,'unexplained effective environment difference'
    env_map[h]=bh;env_records.append({'unit':ui,'tip':h,'base':bh,'cross_unit_environment_identities':imports,'exact_canonical_bytes_match':True,'two_functions_and_exact_source_suffixes_only':True})
   assert set(env_map.values())==set(bv['environments']) and len(env_map)==len(bv['environments'])
  br={r['function']:r for r in bu['functions']};tr={r['function']:r for r in tu['functions']};assert br.keys()==tr.keys()
  for n in br:
   old,new=[r.get('reading_site_evidence',[])for r in [br[n],tr[n]]];assert len(old)==len(new)
   baseline={r['id']:r for r in old};assert len(baseline)==len(old)
   for r in old:assert receipt_id(r)==r['id'],'actual BASE typed receipt preimage mismatch'
   restored=set()
   for r in new:
    assert receipt_id(r)==r['id'],'actual TIP typed receipt preimage mismatch';v=copy.deepcopy(r);v['observation']['environment_sha256']=env_map[v['observation']['environment_sha256']]
    for field,value in v['context'].items():
     if (value,field)in known:v['context'][field]=known[(value,field)]
    v['id']=receipt_id(v);assert baseline.get(v['id'])==v,'non-environment observation or complete receipt changed';restored.add(v['id']);total+=1
    receipt_records.append({'unit':ui,'function':n,'base':v['id'],'tip':r['id'],'all_non_environment_observation_fields_exact':True,'whole_restored_receipt_equals_actual_base':True,'both_typed_id_preimages_verified':True})
   assert restored==set(baseline)
 result={'route':route,'exit':0,'input_diagnostics_sha256':{side:sha((directory/('corpus/resource-diagnostics.json'if route=='corpus'else'resource-diagnostics.json')).read_bytes())for side,directory in dirs.items()},'identity_proof_sha256':sha((P/(route+'-identity-attribution.json')).read_bytes()),'effective_environment_captures':len(env_records),'receipts':total,'environment_records':env_records,'receipt_records':receipt_records,'producer_sources':producers,'source_binding':'open','truth_transport':False,'fields_erased':0,'fields_masked':0,'meaning':'Exact whole canonical environments reconstructed from reviewed two-function/two-source additions and independently proven home-environment identities; typed serde bind ID preimages on both actual receipts match, complete restored receipt sets equal actual BASE. Receipt order changes follow changed IDs. This replays identities, not C extraction or libc behavior.'}
 (P/(route+'-reading-attribution.json')).write_text(json.dumps(result,indent=2)+'\n');print({'route':route,'captures':len(env_records),'receipts':total,'exit':0})
