from pathlib import Path
import subprocess,json,hashlib
p=Path('private/w48-cmt-tracked-interior');read=lambda f:json.loads(Path(f).read_bytes());sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
freeze=read(p/'semantic-freeze.json')['head'];head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
changed=subprocess.check_output(['git','diff','--name-only',freeze,head],text=True).splitlines()
runtime=[x for x in changed if ('/src/' in x and x.endswith('.rs')) or x in ['Cargo.toml','Cargo.lock']]
assert not runtime, runtime
old=json.loads(subprocess.check_output(['git','show',freeze+':providers/contract-load-path.json']))
new=read('providers/contract-load-path.json')
views=lambda d:{c['name']:{k:c[k] for k in ['resource_view','bounds_view','outer_view']} for c in d['crates']}
assert views(old)==views(new)
profile='contract/profiles/cmetrics-counter/source-contracts.json'
old_policy=json.loads(subprocess.check_output(['git','show',freeze+':'+profile]))
new_policy=read(profile)
assert new_policy['bindings'][0]['contract']['sha256']==sha('providers/qcore-provider-cmetrics/contracts.json')
new_policy['bindings'][0]['contract']['sha256']=old_policy['bindings'][0]['contract']['sha256']
assert new_policy==old_policy,'only the required inventory identity may change'
selected={}
for c in new['crates']:
 for vocabulary in ['resource_view','bounds_view','outer_view']:
  for module in c[vocabulary]:
   f='providers/'+c['contracts']+'/'+module
   original=subprocess.check_output(['git','show',freeze+':'+f])
   assert Path(f).read_bytes()==original,f
   selected[f]=sha(f)
manifest=read(p/'inputs/manifest.json');deltas=[{'path':f,'frozen_base_sha256':h,'canonical_tip_sha256':sha(f)} for f,h in manifest.items() if Path(f).is_file() and sha(f)!=h]
added=[f for f in subprocess.check_output(['git','ls-files','providers'],text=True).splitlines() if f.endswith('.qv') and f not in manifest]
expected=['providers/qcore-provider-cmetrics/contracts/cmetrics/map_lifetime.qv','providers/qcore-provider-cmetrics/contracts/cmetrics/opts_lifetime.qv']
# Some canonical assets are outside the native input manifest by design;
# enumerate only additions to the frozen semantic checkpoint separately.
new_assets=subprocess.check_output(['git','diff','--name-only','--diff-filter=A',freeze,head,'--','providers'],text=True).splitlines()
assert sorted(new_assets)==expected,new_assets
for f in expected:assert 'requires resource_contract_v1;' not in Path(f).read_text()
root=read(p/'latest-gate.json');provider=read(p/'provider-api-gate/gate.json')
assert root['exit']==provider['exit']==0 and root['head']==provider['head']==head
receipt={'native_source':freeze,'api_validated_source':head,'runtime_rust_changes_since_native':runtime,'resource_bounds_outer_views_equal':True,'selected_interpreted_source_bytes_equal':selected,'new_canonical_models':[{ 'path':f,'sha256':sha(f)} for f in expected],'root_gate':root,'provider_gate':'provider-api-gate/gate.json','source_correspondence':'reviewed premise 28; no generated C refinement proof','shipped_counter_reset_policy_selection_unchanged':True,'trusted_api_adoption':'Open'}
(p/'api-source-validation.json').write_text(json.dumps(receipt,indent=2)+'\n')
delta={'snapshot_entries':len(manifest),'intended_canonical_deltas':deltas,'new_canonical_models':receipt['new_canonical_models'],'native_pair_uses_same_frozen_snapshot':True,'resource_bounds_outer_views_unchanged':True,'basis':'Frozen native invocations; runtime source equality; full current-source root/provider gates and actual loader refusal controls','new_model_declarations':['cfl_list_init','cmt_map_create','cmt_map_metric_get','cmt_map_metric_get_val','cmt_map_metric_destroy','cmt_map_metrics_expire','cmt_map_destroy','cmt_opts_init','cmt_opts_exit'],'no_new_source_pin_or_trusted_call_permission':True}
(p/'contract-model-input-delta.json').write_text(json.dumps(delta,indent=2)+'\n')
q=read(p/'qv-correspondence.json');q['final_canonical_loader_and_corpus_control']={'source':head,'gate':root['path'],'units':['qcore-contract::test::tracked_self_init_model','qcore-contract::test::cmetrics_api_models','qcore-compiler::test::contract_loader','qcore-compiler::test::qcore_contract_corpus'],'note':'Current source gate validates all canonical models; native initializer checkpoint is recorded independently.'};q['api_models']=receipt['new_canonical_models'];q['api_lean']='QCore.CmetricsApiContracts / AuditPointerStore';q['api_adoption']='Open';(p/'qv-correspondence.json').write_text(json.dumps(q,indent=2)+'\n')
print(json.dumps(receipt,indent=2))
