#!/usr/bin/env python3
"""Retain complete native semantic/body changes and exact raw C coordinates."""
import argparse,collections,gzip,hashlib,json,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('comparison',type=Path);p.add_argument('--source',type=Path,required=True);a=p.parse_args();out=a.comparison.resolve();assert '/private/' in str(out)
sha=lambda b:hashlib.sha256(b).hexdigest();read=lambda side:json.loads(gzip.decompress((out/side/'resource-diagnostics.json.gz').read_bytes()))
docs={s:read(s) for s in ['base','tip']};moved=json.loads((out/'full-semantic-row-differences.json').read_bytes())
def identity():return {'head':subprocess.check_output(['git','-C',str(a.source),'rev-parse','HEAD'],text=True).strip(),'tracked_delta_sha256':sha(subprocess.check_output(['git','-C',str(a.source),'diff','--no-ext-diff','--binary','HEAD']))}
before_identity=identity();tracked=subprocess.check_output(['git','-C',str(a.source),'ls-files'],text=True).splitlines();prefixes={'chunkio':'lib/chunkio/src/','monkey-libevent':'lib/monkey/','librdkafka':'lib/librdkafka-2.15.0/src/','cmetrics':'lib/cmetrics/src/','ctraces':'lib/ctraces/src/','fluent-bit':''}
classified=[]
for pair in moved:
 old,new=pair['before'],pair['after'];counts=[collections.Counter(json.dumps(d,sort_keys=True) for d in x.get('diagnostics',[])) for x in [old,new]]
 removed,added=[[json.loads(d) for d in x.elements()] for x in [counts[0]-counts[1],counts[1]-counts[0]]];observations=[]
 for d in removed+added:
  origin=d.get('origin')
  if not origin or ':' not in origin:continue
  spelling,number=origin.rsplit(':',1)
  if not number.isdigit():continue
  candidates=[x for x in tracked if x==spelling or x.endswith('/'+spelling)];preferred=[x for x in candidates if x.startswith(prefixes.get(pair['key'][0],''))];candidates=preferred or candidates
  if len(candidates)!=1:observations.append({'origin':origin,'unresolved_source_candidates':candidates});continue
  path=a.source/candidates[0];raw=path.read_bytes();lines=raw.decode().splitlines();line=int(number)
  observations.append({'origin':origin,'source_path':candidates[0],'file_sha256':sha(raw),'excerpt':'\n'.join(lines[max(0,line-3):line+3])})
 classified.append({'key':pair['key'],'before':old['status'],'after':new['status'],'removed':removed,'added':added,'source_observations':observations})
known={}
for unit in docs['base']['units']:
 e=unit.get('contract_body_evidence',{})
 for name,methods in e.get('validated',{}).items():
  value={'methods':methods,'clauses':e.get('clauses',{}).get(name)};known.setdefault(name,[])
  if value not in known[name]:known[name].append(value)
body=[]
for i,(left,right) in enumerate(zip(docs['base']['units'],docs['tip']['units'])):
 assert (left['library'],left['file'])==(right['library'],right['file'])
 x,y=[u.get('contract_body_evidence',{}) for u in [left,right]]
 if x==y:continue
 retained=[];changed=[];added=[]
 for name,methods in x.get('validated',{}).items():
  same=y.get('validated',{}).get(name)==methods and y.get('clauses',{}).get(name)==x.get('clauses',{}).get(name)
  (retained if same else changed).append(name)
 for name,methods in y.get('validated',{}).items():
  if name in x.get('validated',{}):continue
  payload={'methods':methods,'clauses':y.get('clauses',{}).get(name)};added.append({'function':name,'payload':payload,'existed_in_baseline_units':payload in known.get(name,[])})
 body.append({'index':i,'library':left['library'],'file':left['file'],'before':x,'after':y,'prior_validated_payloads_retained':retained,'prior_validated_payloads_changed':changed,'new_replay_payloads':added})
assert before_identity==identity()
result={'source_root':str(a.source.resolve()),'source_identity_before_after':before_identity,'rows':classified,'body_evidence_changes':body,'meaning':'Exact complete changes and source coordinates for review; C snippets do not discharge custody obligations. Replay/header multiplicity is not distinct native coverage.'}
(out/'source-body-attribution.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'semantic_rows':len(classified),'body_units':len(body),'prior_methods_changed':sum(len(x['prior_validated_payloads_changed']) for x in body)}))
