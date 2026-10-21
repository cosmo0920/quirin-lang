"""Derive exact metadata substitutions from held inputs, never raw row diffs."""
import argparse,hashlib,json
from pathlib import Path
root=Path.cwd();report=root/'docs/reports/plugin-premise-decomposition';p=argparse.ArgumentParser();p.add_argument('base_inputs',type=Path);p.add_argument('tip_inputs',type=Path);p.add_argument('environments',type=Path);p.add_argument('out',type=Path);a=p.parse_args();sha=lambda b:hashlib.sha256(b).hexdigest();read=lambda p:json.loads(p.read_bytes());known={};reasons={}
def add(new,old,reason):
 if new==old:return
 assert isinstance(new,str) and isinstance(old,str)
 if new in known:assert known[new]==old
 known[new]=old;reasons.setdefault(sha(new.encode()),[]).append(reason)
idx=read(report/'bindings/index.json')
for item in idx['modules']:
 relative=Path(item['path']);old=(a.base_inputs/relative).read_bytes();new=(a.tip_inputs/relative).read_bytes();reconstructed=b''.join(line for line in new.splitlines(keepends=True) if not(line.lstrip().startswith(b'requires resource_hypothesis(plugin_') or line.lstrip().startswith(b'requires resource_premise_decomposition(rk_plugin_callback_closure,') or line.startswith(b'// qcore-premise-binding-manifest-v1 ')));assert reconstructed==old
 add(new.decode(),old.decode(),{'kind':'exact-six-module-byte-inverse','path':str(relative)});add(sha(new),sha(old),{'kind':'raw-source-sha256','path':str(relative)})
selected=read(report/'selected-source-identities.json')['selected'];lock_paths=sorted({row['lock'] for row in selected if row['original_entries']})
def entries(doc):
 rows=doc['pinned'];return {r['function']:r for r in rows} if isinstance(rows,list) else rows
for relative in lock_paths:
 old_raw=(a.base_inputs/relative).read_bytes();new_raw=(a.tip_inputs/relative).read_bytes();old=read(a.base_inputs/relative);new=read(a.tip_inputs/relative);oe,ne=entries(old),entries(new);assert oe.keys()==ne.keys();allowed={row['declaration'] for row in selected if row['lock']==relative and row['original_entries']};restored=json.loads(new_raw)
 re=entries(restored)
 for name,before in oe.items():
  after=ne[name]
  if before==after:continue
  assert name in allowed
  rewritten=dict(after);rewritten['contract_sha256']=before['contract_sha256'];assert rewritten==before
  add(after['contract_sha256'],before['contract_sha256'],{'kind':'official-selected-contract-pin','path':relative,'declaration':name,'complete-C-source-identities':'exact'})
  re[name]['contract_sha256']=before['contract_sha256']
 assert restored==old
 add(sha(new_raw),sha(old_raw),{'kind':'complete-lock-file-sha256','path':relative})
roots=sorted([str(a.base_inputs.resolve()),str(a.tip_inputs.resolve())],key=len,reverse=True)
def rewrite(value):
 if isinstance(value,str):
  value=known.get(value,value)
  for prefix in roots:value=value.replace(prefix,'$INPUTS')
  return value
 if isinstance(value,list):return [rewrite(x) for x in value]
 if isinstance(value,dict):
  pairs=[(rewrite(k),rewrite(v)) for k,v in value.items()];assert len(dict(pairs))==len(pairs);return dict(pairs)
 return value
# Whole file hashes are admitted only after every field reconstructs exactly.
for relative in ['providers/qcore-provider-librdkafka/contracts.json','providers/contract-load-path.json']:
 before=(a.base_inputs/relative).read_bytes();after=(a.tip_inputs/relative).read_bytes();assert rewrite(json.loads(after))==rewrite(json.loads(before));add(sha(after),sha(before),{'kind':'independently-reconstructed-inventory-file','path':relative})
assert not (a.base_inputs/'providers/quirin-provider-fluent-bit/contracts.json').exists() and not (a.tip_inputs/'providers/quirin-provider-fluent-bit/contracts.json').exists() # This source-only family has no provider inventory file.
hashes={side:read(a.environments/(side+'-environment-hashes.json')) for side in ['base','tip']};env_bytes={side:(a.environments/(side+'-environment.json')).read_bytes() for side in hashes};env={side:json.loads(raw) for side,raw in env_bytes.items()}
for side,raw in env_bytes.items():assert sha(raw)==hashes[side]['environment_sha256']
# The selected memory module is officially elaborated by the real loader.
# Its retained flat text is a distinct input from the written #inherit source.
key='qcore-provider-librdkafka:librdkafka/memory.qv'
old_flat=env['base']['source_assumptions'][key];new_flat=env['tip']['source_assumptions'][key]
inverse=''.join(line for line in new_flat.splitlines(keepends=True) if not(line.lstrip().startswith('requires resource_hypothesis(plugin_') or line.lstrip().startswith('requires resource_premise_decomposition(rk_plugin_callback_closure,') or line.startswith('// qcore-premise-binding-manifest-v1 ')))
assert inverse==old_flat,'official flat-memory source must reconstruct byte-for-byte'
add(new_flat,old_flat,{'kind':'exact-neutral-metadata-inverse-of-officially-elaborated-memory-source','module':key,'base_sha256':sha(old_flat.encode()),'tip_sha256':sha(new_flat.encode())})
add(sha(new_flat.encode()),sha(old_flat.encode()),{'kind':'official-flat-memory-source-sha256','module':key})
if rewrite(env['base'])!=rewrite(env['tip']):
 import runpy
 differences=runpy.run_path(str(root/'docs/reports/contract-hypothesis-graph-v1/scripts/compare.py'))['differences']
 a.out.with_suffix('.root-residuals.json').write_text(json.dumps(differences(rewrite(env['base']),rewrite(env['tip'])),indent=2)+'\n')
 raise AssertionError('unexplained full root environment field; no hash waiver')
assert hashes['base']['function_table_sha256']==hashes['tip']['function_table_sha256']
for key in ['environment_sha256','source_assumptions_sha256','function_table_sha256']:add(hashes['tip'][key],hashes['base'][key],{'kind':'held-complete-root-environment-reconstruction','field':key,'base_environment_sha256':sha(env_bytes['base']),'tip_environment_sha256':sha(env_bytes['tip'])})
result={'status':'exact independently held input/environment reconstruction','exact_strings':known,'justifications_by_new_string_sha256':reasons,'source_binding':'open','full_environment_fields_equal_after_exact_source_inclusion_and_snapshot_root_restoration':True,'function_table_equal':True,'input_roots':roots,'environment_hashes':hashes};a.out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'substitutions':len(known),'source_binding':'open'}))
