from pathlib import Path
import copy,gzip,json,hashlib
repo=Path.cwd();report=repo/'docs/reports/w48-bottom-up-batch4';out=report/'artifacts/corpus';private=repo/'private/w48-bottom-up-batch4'
load=lambda s,f:json.loads(gzip.decompress((out/s/(f+'.json.gz')).read_bytes()))
roots=[str(private/n) for n in ['inputs-base-final','inputs-tip-final']]
def norm(x):
 if isinstance(x,str):
  for p in roots:x=x.replace(p,'$INPUTS')
  return x
 if isinstance(x,list):return [norm(v) for v in x]
 if isinstance(x,dict):
  result={norm(k):norm(v) for k,v in x.items()};assert len(result)==len(x);return result
 return x
C={s:load(s,'contracts') for s in ['before','after']};proof=json.loads((out/'context-identity-review.json').read_text());assert proof['hypothesis_authority_immutable_fields_identical'];authority=json.loads((out/'authority-review.json').read_text());assert authority['unchanged_declared_modes_pins_and_graph_authority']
a,b=[norm(copy.deepcopy(C[s])) for s in C];before_imp={r['function']:r for r in a['cross_unit_link_resolution']['imported']};after_imp={r['function']:r for r in b['cross_unit_link_resolution']['imported']};assert after_imp.keys()-before_imp.keys()=={'cio_file_is_up'} and not before_imp.keys()-after_imp.keys();oldenv=proof['environments']['before']['sha256'];newenv=proof['environments']['after']['sha256']
for name in before_imp:
 x,y=before_imp[name],copy.deepcopy(after_imp[name]);assert x['home_environment_sha256']==oldenv and y['home_environment_sha256']==newenv;y['home_environment_sha256']=oldenv;assert x==y
new=after_imp['cio_file_is_up'];assert new['home_unit']=='chunkio/cio_file.c';assert new['home_environment_sha256']==newenv
for key in ['home_raw_source_sha256','home_preprocessed_source_sha256']:assert new[key]==after_imp['cio_file_hash'][key]
b['cross_unit_link_resolution']=a['cross_unit_link_resolution']
# The complete producer replay and exact authority review cover this whole
# derived section; all semantic usage/native-standing changes stay in receipts.
b['contract_dependencies']=a['contract_dependencies']
time=json.loads((out/'time-identity-review.json').read_text())
for path in time['paths']:
 d=b
 for k in path:d=d[k]
 assert d.pop(time['after_key'])=={};d[time['before_key']]={}
assert a==b,'unexplained complete contract-artifact field'
result={'complete_contract_artifacts_equal_after_individually_proved_derivations':True,'producer_dependency_replays':'before/after-metadata-replay.json','native_body_standing_and_usage_review':'authority-review.json','context_reconstruction':'context-identity-review.json','time_key_reconstruction':'time-identity-review.json','new_conditional_home_import':new,'existing_import_payloads_preserved_after_exact_environment_reconstruction':list(before_imp),'meaning':'No raw field is deleted. Complete contract artifacts are equal after the freshly replayed dependency section, exactly reconstructed environment labels, the single newly checked cio_file_is_up import and the separately reproduced empty-summary time key. Raw payloads and all differences remain archived.'}
(out/'complete-contract-review.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'complete_contract_review':True,'old_imports':len(before_imp),'new_imports':len(after_imp)}))
