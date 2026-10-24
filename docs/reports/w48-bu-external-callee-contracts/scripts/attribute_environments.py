"""Exact complete typed environment preimages for the two reviewed additions.

Run after own TIP exports. No digest masking, deleted fields or C-source truth
claim: literal source text changes and exactly two function/revision entries
are independently checked against held files and actual Rust-exported bytes.
"""
from pathlib import Path
import json,hashlib,copy,subprocess
import argparse
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--evidence-root',type=Path,default=Path(__file__).parent);args=parser.parse_args()
P=args.evidence_root.resolve()
sha=lambda b:hashlib.sha256(b).hexdigest()
serial=lambda v:json.dumps(v,ensure_ascii=False,separators=(',',':')).encode()
records=[]
for route in ['corpus','system','slice']:
 for side in ['base','tip']:
  exit_file=P/(side+'-environments')/route/'exit';assert exit_file.exists() and exit_file.read_text().strip()=='0',(side,route,'export incomplete or failed')
 bfile=P/'base-environments'/route/'base-environment.json';tfile=P/'tip-environments'/route/'tip-environment.json'
 braw=bfile.read_bytes();traw=tfile.read_bytes();b=json.loads(braw);t=json.loads(traw);expected=copy.deepcopy(b)
 source_pairs=[]
 for rel in ['libc/threads.qv','libc/unistd.qv']:
  keys=[k for k in b['source_assumptions'] if k.endswith('/'+rel)];assert len(keys)==1;key=keys[0]
  old=(P/'base-source-archive/providers/quirin-provider-libc/contracts'/rel).read_bytes().decode('utf-8');new=(P/'tip-source-archive/providers/quirin-provider-libc/contracts'/rel).read_bytes().decode('utf-8')
  assert b['source_assumptions'][key]==old and t['source_assumptions'][key]==new
  assert new.startswith(old),rel
  assert new[len(old):].startswith('\n// ## Card W48-BU-EXTERNAL-CALLEE-CONTRACTS:'),rel
  expected['source_assumptions'][key]=new
  source_pairs.append({'key':key,'relative':'providers/quirin-provider-libc/contracts/'+rel,'base_sha256':sha(old.encode()),'tip_sha256':sha(new.encode()),'appended_sha256':sha(new[len(old):].encode())})
 for name,template in [('writev','write'),('thrd_current','pthread_self')]:
  assert name not in b['functions'] and name not in b['revision']
  expected['functions'][name]=copy.deepcopy(b['functions'][template]);expected['revision'][name]=copy.deepcopy(b['revision'][template])
 for key in ['functions','revision']:expected[key]=dict(sorted(expected[key].items()))
 assert expected==t,'complete environment changed beyond the two reviewed additions and exact source texts'
 assert serial(expected)==traw,'typed serde byte preimage mismatch; do not infer serialization order from a digest'
 assert serial(b)==braw
 hashes={side:json.loads((P/(side+'-environments')/route/(side+'-environment-hashes.json')).read_text()) for side in ['base','tip']}
 for side,env,raw in [('base',b,braw),('tip',t,traw)]:
  assert sha(raw)==hashes[side]['environment_sha256']
  assert sha(serial(env['functions']))==hashes[side]['function_table_sha256']
  assert sha(serial(env['source_assumptions']))==hashes[side]['source_assumptions_sha256']
 negative=copy.deepcopy(expected);negative['source_assumptions'][source_pairs[0]['key']]+='\n';assert sha(serial(negative))!=sha(traw)
 negative_table=copy.deepcopy(expected['functions']);negative_table['writev']['parameters'].pop();assert sha(serial(negative_table))!=hashes['tip']['function_table_sha256']
 records.append({'route':route,'counts':{side:{k:len(v) for k,v in env.items()} for side,env in [('base',b),('tip',t)]},'source_pairs':source_pairs,'hashes':hashes,'new_functions':{k:t['functions'][k] for k in ['writev','thrd_current']},'new_revision_entries':{k:t['revision'][k] for k in ['writev','thrd_current']},'complete_environment_equal_after_exact_preimage_reconstruction':True,'typed_byte_equality':True,'negative_comment_and_wrong_arity_sensitivity':True})
result={'records':records,'fields_deleted':0,'fields_masked':0,'meaning':'Complete actual exported typed environments; equality after exactly two new ordinary function entries, two flexible revision entries and the exact two source-file suffixes. Does not prove libc implementations, source binding, C extraction or Rust-Lean correspondence.'}
(P/'environment-attribution.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'routes':len(records),'typed_preimages_verified':True}))
