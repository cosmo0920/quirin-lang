#!/usr/bin/env python3
"""Reconstruct each raw metadata hash, then attribute exact bounded changes."""
import argparse,gzip,hashlib,json,runpy
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
PRIVATE=ROOT/'private/premise-decomposition-cycle1'
p=argparse.ArgumentParser();p.add_argument('route');p.add_argument('--comparison',type=Path);p.add_argument('--raw-root',type=Path);p.add_argument('--environment-directory',type=Path);a=p.parse_args()
out=a.comparison or ROOT/'docs/reports/premise-decomposition-cycle1/measurements'/a.route
raw=a.raw_root or PRIVATE/'measure-1'/a.route
environment_directory=a.environment_directory or PRIVATE
sha=lambda b:hashlib.sha256(b).hexdigest()
read=lambda p:json.loads(p.read_bytes())
inputs=PRIVATE/'base-inputs'
relative='providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv'
old_source=(inputs/relative).read_text();new_source=(ROOT/relative).read_text()
lock='providers/qcore-provider-librdkafka/contracts.lock'
old_lock=(inputs/lock).read_bytes();new_lock=(ROOT/lock).read_bytes()
selected=lambda b:next(r['contract_sha256'] for r in json.loads(b)['pinned'] if r['function']=='cJSON_DetachItemViaPointer')
old_digest,new_digest=selected(old_lock),selected(new_lock)
assert new_lock.replace(new_digest.encode(),old_digest.encode())==old_lock
known={new_source:old_source,new_digest:old_digest,sha(new_source.encode()):sha(old_source.encode()),sha(new_lock):sha(old_lock)}
roots=[str(inputs),str(ROOT),'/home/cosmo/.codex/worktrees/w48-bottom-up-batch4/qcore-c'];roots.sort(key=len,reverse=True)
def norm(v):
 if isinstance(v,str):
  v=known.get(v,v)
  for root in roots:v=v.replace(root,'$INPUTS')
  return v
 if isinstance(v,list):
  return [norm(x) for x in v if not(isinstance(x,dict) and x.get('name')=='detach_representation_atoms')]
 if isinstance(v,dict):
  pairs=[(norm(k),norm(x)) for k,x in v.items()];assert len(dict(pairs))==len(pairs);return dict(pairs)
 return v
hashes={side:read(environment_directory/(side+'-environment-hashes.json')) for side in ['base','tip']}
env={side:(environment_directory/(side+'-environment.json')).read_bytes() for side in hashes}
for side in hashes:assert sha(env[side])==hashes[side]['environment_sha256']
assert norm(json.loads(env['base']))==norm(json.loads(env['tip'])), 'full environment must differ only by exact migration and roots'
for key in ['environment_sha256','source_assumptions_sha256','function_table_sha256']:known[hashes['tip'][key]]=hashes['base'][key]
configs={};inventory=[]
for side in ['base','tip']:
 doc=read(raw/side/'resource-diagnostics.json');configs[side]=[]
 for u in doc['units']:
  cfg=u.get('compile_configuration')
  if cfg is None:configs[side].append(None);continue
  cfg=dict(cfg);actual=cfg.pop('inventory_sha256')
  computed=sha(json.dumps(cfg,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode())
  assert computed==actual,(side,actual,computed)
  configs[side].append(cfg);inventory.append({'side':side,'actual':actual,'reconstructed':computed})
assert norm(configs['base'])==norm(configs['tip']),'every complete compile configuration must match after exact transforms'
for side in ['base','tip']:
 doc=read(raw/side/'resource-diagnostics.json')
 if side=='base':base_doc=doc
 else:
  for l,r in zip(base_doc['units'],doc['units']):
   if 'compile_configuration' in l:known[r['compile_configuration']['inventory_sha256']]=l['compile_configuration']['inventory_sha256']
diffs=json.loads(gzip.decompress((out/'erased-migration-normalized-differences.json.gz').read_bytes()))
remaining={};attributed=[]
for label,changes in diffs.items():
 remaining[label]=[]
 for change in changes:
  if 'before' in change and 'after' in change and norm(change['before'])==norm(change['after']):attributed.append({'artifact':label,**change})
  else:remaining[label].append(change)
(out/'metadata-attribution.json').write_text(json.dumps({'lock_file':{'base_sha256':sha(old_lock),'tip_sha256':sha(new_lock),'exact_single_declaration_digest_substitution_restores_base_bytes':True},'environment':hashes,'complete_environment_equal_after_exact_migration_and_root_restoration':True,'all_compile_configurations_equal_after_exact_source_map_hash_and_root_restoration':True,'inventory_reconstructions':inventory,'attributed_leaf_counts':{k:sum(x['artifact']==k for x in attributed) for k in diffs},'remaining_leaf_counts':{k:len(v) for k,v in remaining.items()},'boundary':'Raw records remain archived. Exact strings/hashes and root spellings only; no blanket hash/path filtering, no semantic field is discarded.'},indent=2)+'\n')
(out/'metadata-attributed-differences.json.gz').write_bytes(gzip.compress((json.dumps(attributed,indent=2)+'\n').encode(),mtime=0))
(out/'remaining-differences.json.gz').write_bytes(gzip.compress((json.dumps(remaining,indent=2)+'\n').encode(),mtime=0))
for side in env:(out/(side+'-environment.json.gz')).write_bytes(gzip.compress(env[side],mtime=0))
print(json.dumps({k:len(v) for k,v in remaining.items()}))
