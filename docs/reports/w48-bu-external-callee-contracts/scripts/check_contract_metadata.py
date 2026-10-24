"""Reconstruct full revision/source metadata from reviewed source and lock data."""
from pathlib import Path
import argparse,json,hashlib,copy
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--evidence-root',type=Path,default=Path(__file__).parent);parser.add_argument('--routes',nargs='+',choices=['corpus','system','slice'],default=['corpus','system','slice']);args=parser.parse_args();P=args.evidence_root.resolve()
read=lambda f:json.loads(f.read_bytes());sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
lock=read(P/'tip-source-archive/providers/quirin-provider-libc/contracts.lock');flexible={r['function']:r for r in lock['flexible']}
records=[]
for route in args.routes:
 receipt=P/'tip-1'/route/(route+'.exit');assert receipt.exists() and receipt.read_text().strip()=='0',(route,'native route incomplete or failed; do not attribute intermediate metadata')
 rel='corpus/resource-diagnostics.contracts.json' if route=='corpus' else 'resource-diagnostics.contracts.json'
 b,t=[read(P/d/route/rel) for d in ['base-3','tip-1']];r=copy.deepcopy(b['contract_revision']);added=[]
 for name,template,module in [('writev','write','libc/unistd.qv'),('thrd_current','pthread_self','libc/threads.qv')]:
  assert name not in r['contracts'];entry=copy.deepcopy(r['contracts'][template]);entry['contract_sha256']=flexible[name]['contract_sha256'];r['contracts'][name]=entry;added.append({'function':name,'entry':entry})
 standing=r['assumptions']['standing'];assert len(standing)==1 and standing[0]['name']=='libc-c-standard-abi'
 standing[0]['functions']=sorted(standing[0]['functions']+['writev','thrd_current']);standing[0]['contracts']+=2;r['counts']['standing_libc']+=2
 keys=[k for k in r['locks'] if k.endswith('/providers/quirin-provider-libc/contracts.lock')];assert len(keys)==1;r['locks'][keys[0]]['sha256']=sha(P/'tip-source-archive/providers/quirin-provider-libc/contracts.lock')
 assert r==t['contract_revision'],'revision metadata changed beyond exact new standing entries and lock bytes'
 source_key='sources'if route=='corpus'else'contracts'
 expected=copy.deepcopy(b[source_key]);changed=[]
 for module in ['libc/threads.qv','libc/unistd.qv']:
  keys=[k for k in expected if k.endswith('/'+module)];assert len(keys)==1;k=keys[0]
  old=(P/'base-source-archive/providers/quirin-provider-libc/contracts'/module).read_text();new=(P/'tip-source-archive/providers/quirin-provider-libc/contracts'/module).read_text();assert expected[k]==old;expected[k]=new;changed.append(k)
 assert expected==t[source_key],'raw source metadata has unexplained changes'
 premise_reconstruction=False
 if 'contract_premises'in b:
  expected_premises=copy.deepcopy(b['contract_premises']);graph={'version':1,'atoms':{},'premises':{}};graph_hash=hashlib.sha256(json.dumps(graph,separators=(',',':')).encode()).hexdigest()
  for name in ['writev','thrd_current']:
   assert name not in expected_premises['$hypothesis_graphs'] and name not in expected_premises['$hypothesis_graph_digests']
   expected_premises['$hypothesis_graphs'][name]=graph;expected_premises['$hypothesis_graph_digests'][name]=graph_hash
  assert expected_premises==t['contract_premises'];premise_reconstruction=True
 changed_sections={'contract_revision',source_key,'contract_premises','contract_dependencies','callback_members','cross_unit_link_resolution'}
 assert b.keys()==t.keys()
 unchanged=[k for k in b if k not in changed_sections]
 for k in unchanged:assert b[k]==t[k],(route,k,'unexpected metadata change')
 records.append({'route':route,'new_revision_entries':added,'standing_before':b['contract_revision']['counts']['standing_libc'],'standing_after':t['contract_revision']['counts']['standing_libc'],'complete_revision_metadata_reconstructed':True,'complete_sources_reconstructed':True,'source_keys_changed':changed,'input_metadata_sha256':{side:sha(P/directory/route/rel)for side,directory in [('base','base-3'),('tip','tip-1')]},'unchanged_other_fields':unchanged,'new_empty_graph_metadata_reconstructed':premise_reconstruction,'scope':'Contract dependencies and cross-unit imports are separately compared and attributed.'})
for record in records:(P/(record['route']+'-contract-metadata-attribution.json')).write_text(json.dumps({'exit':0,'record':record},indent=2)+'\n')
(P/('contract-metadata-attribution.json' if len(records)==3 else 'contract-metadata-preflight.json')).write_text(json.dumps({'exit':0,'records':records},indent=2)+'\n');print('verified routes',len(records))
