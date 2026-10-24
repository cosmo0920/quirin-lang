from pathlib import Path
import json,re,shutil,gzip,subprocess
root=Path.cwd();p=root/'private/w48-cmt-member-storage';r=root/'docs/reports/w48-cmt-list-self-init';out=r/'integration'
read=lambda f:json.loads(Path(f).read_text());write=lambda f,x:Path(f).write_text(json.dumps(x,indent=2)+'\n')
latest=read(p/'latest-integration-gate.json');assert latest['exit']==0
d=Path(latest['path']);g=read(d/'gate-run.json');assert g['exit']==0 and g['head_after']==g['head'] and g['clean_after'];assert all(x['exit']==0 for x in g['steps'])
rust=read(d/'rust-binaries/summary.json');assert rust['ok'] and not rust['units_failed']
lean=int(re.search(r'Passed (\d+) Lean audits',(d/'lean-audits.log').read_text()).group(1));python=int(re.search(r'Ran (\d+) tests',(d/'python-tests.log').read_text()).group(1))
summaries=[tuple(map(int,x)) for x in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',(d/'provider-rust.log').read_text())];assert summaries and all(x[1]==0 for x in summaries)
provider={k:sum(x[i] for x in summaries) for i,k in enumerate(['passed','failed','ignored','measured','filtered'])};provider['summaries']=len(summaries)
provider_python=int(re.search(r'Ran (\d+) tests',(d/'provider-python.log').read_text()).group(1));banner=(d/'clang-version.txt').read_text().splitlines()[0]
ready=read(p/'integration-ready.json');g.update(merge_commit=subprocess.check_output(['git','rev-parse',ready['merge_commit']],text=True).strip(),main_base=ready['main_base'],card_measured_tip=read(r/'native-comparison.json')['measured_tip'],rust=rust,lean_audits=lean,python_tests=python,provider_rust=provider,provider_python_tests=provider_python,packages_verified=[s['name'] for s in g['steps'] if s['name'].startswith('verify-')],native_scope='Card-native comparisons remain scoped to 99457f141; no fresh combined-main native measurement is claimed.')
write(out/'standing-gate.json',g)
for name in ['clang-version.txt','clang-version.exit']:shutil.copyfile(d/name,out/name)
note=f"Combined CMetrics/main source {g['head']} (semantic merge {g['merge_commit']}, main base {g['main_base']}) passes the full root and provider gate under the acquired shared validation lock. Root {rust['totals']['passed']} passed/0 failed/{rust['totals']['ignored']} ignored ({rust['units_run']} units/{rust['totals']['summaries']} summaries); {lean} Lean audits, {python} Python. Provider {provider['passed']} passed/{provider['failed']} failed/{provider['ignored']} ignored, {provider_python} Python; inventory, formatting, Clippy, package build and all five verifies pass. Every command exits 0. Actual Clang recording is adjacent, from this worktree and this run. Logs: {d}. Complete commands/exits/source guards: standing-gate.json. Counts differ from the isolated card gate and are published separately. Card-native figures are not a combined-main measurement."
if 'provider_evidence_reuse' in g:
 note+=' Provider commands were measured at '+g['provider_evidence_reuse']['measured_source']+' in the retained first integration attempt and reused after exact confirmation that all provider/runtime inputs are unchanged; only three test expectations and documentation differ. They are not claimed as fresh commands at the repaired root tip.'
gate={'note':note,'workspace_tests':rust['totals']['passed'],'provider_tests':provider['passed'],'python_evidence_validator_tests':python,'reevaluation_probe_exit':0,'reevaluation_followup_exit':0,'cargo_fmt':'clean','cargo_clippy':'clean','standing_gate':'complete on combined committed source '+g['head']+'; root and provider gates','lean_audits':lean,'clang':'present: '+banner};write(out/'gate.json',gate)
logs=out/'logs';logs.mkdir(exist_ok=True)
for f in d.iterdir():
 if f.is_file() and f.suffix in ['.log','.txt','.exit','.json']:(logs/(f.name+'.gz')).write_bytes(gzip.compress(f.read_bytes(),mtime=0))
write(out/'rust-summary.json',rust)
status=root/'docs/STATUS.md';s=status.read_text()
# Derive current canonical module counts from tracked files under indexed roots.
index=read(root/'contract/corpus.json');tracked=subprocess.check_output(['git','ls-files','providers'],text=True).splitlines();counts={}
for entry in index['roots']:
 directory=(root/'contract'/entry).resolve();prefix=str(directory.relative_to(root))+'/'
 counts[prefix]=sum(name.startswith(prefix) and name.endswith('.qv') for name in tracked)
canonical=sum(counts.values());executable=sum(n for name,n in counts.items() if name.startswith('providers/qcore-provider-'));shared=canonical-executable
write(out/'canonical-contract-inventory.json',{'source':g['head'],'tracked_indexed_roots':counts,'total':canonical,'five_executable_provider_roots':executable,'six_shared_application_roots':shared})
s,count=re.subn(r'The five provider crates own \*\*\d+\*\* canonical contract modules; \*\*\d+\*\*',f'The five provider crates own **{executable}** canonical contract modules; **{shared}**',s,count=1);assert count==1
s,count=re.subn(r'indexes all \*\*\d+\*\*',f'indexes all **{canonical}**',s,count=1);assert count==1
start=s.index('## Latest validated gate\n');end=s.index('The five provider crates own',start)
replacement=f"## Latest validated gate\n\nThe [combined CMetrics/main gate](reports/w48-cmt-list-self-init/integration/gate.json) validates committed source `{g['head']}` after semantic merge `{g['merge_commit']}`. Root Rust: **{rust['totals']['passed']:,} passed / 0 failed / {rust['totals']['ignored']} ignored**, {rust['units_run']} units / {rust['totals']['summaries']} summaries. **{lean} Lean audits** pass under pinned 4.35.0, and **{python} Python tests** pass against the committed matrix. Formatting, Clippy and both reevaluation examples exit 0. Providers pass **{provider['passed']} Rust / 0 failed / {provider['ignored']} ignored**, **{provider_python} Python**, inventory, package build and all five package verifications. Every gate step exits 0; actual Clang 21.1.8, commands, source guards and private logs are in the receipt. Isolated CMetrics resource/NULL/runtime comparisons remain at their named measured tips; this source gate supplies no new combined-main native count or discharge of source premises. Final regenerated-matrix Python confirmation is recorded separately.\n\n"
if 'provider_evidence_reuse' in g:
 replacement+='Provider checks above were measured at `'+g['provider_evidence_reuse']['measured_source']+'` and reused after exact runtime/provider source equality; repaired test expectations and documentation do not change those inputs. The repaired root gate and its adjacent Clang recording are fresh.\n\n'
status.write_text(s[:start]+replacement+s[end:])
f=r/'README.md';s=f.read_text();s+='\nThe [combined-source integration gate](integration/gate.json) passes on `'+g['head']+'`, after semantic merge `'+g['merge_commit']+'`: '+str(rust['totals']['passed'])+' Rust tests, '+str(lean)+' Lean audits, '+str(python)+' Python tests, '+str(provider['passed'])+' provider Rust tests and '+str(provider_python)+' provider Python tests; formatting, Clippy, both examples, inventory and all five package checks exit 0. This gate is distinct from the isolated card gate. The card-native measurements above are not re-labelled as combined-main results.\n';f.write_text(s)
if 'provider_evidence_reuse' in g:
 with (r/'README.md').open('a') as stream:stream.write('Provider checks were measured at `'+g['provider_evidence_reuse']['measured_source']+'` in the first integration attempt and reused after exact runtime/provider source equality; the repaired root gate and Clang recording are fresh. The first combined attempt retained two Rust expectation failures and one Python inheritance-list failure. The repair updates only those three tests and pins the exact historical rule37 source-prefix transition; runtime, contract and Lean semantics remain unchanged.\n')
# Retain the complete failed combined attempt beside the repaired acceptance.
failed=out/'attempts/20261024T092516Z';failed.mkdir(parents=True,exist_ok=True)
first=p/'integration-gate/20261024T092516Z'
for file in first.iterdir():
 if file.is_file() and file.suffix in ['.log','.txt','.exit','.json']:(failed/(file.name+'.gz')).write_bytes(gzip.compress(file.read_bytes(),mtime=0))
write(failed/'rust-summary.json',read(first/'rust-binaries/summary.json'))
for name in ['repair_integration_expectations.py','repair_semantic_checkpoint.py','integration_repair_campaign.py','integration_root_gate.py']:
 shutil.copyfile(p/name,r/'scripts'/name)
rec=read(out/'reconciliation.json');rec['remaining']=['Regenerate matrix with actual combined gate, commit it, run committed-matrix Python acceptance, advance main and push configured origin.'];rec['integration_gate']='standing-gate.json';write(out/'reconciliation.json',rec)
shutil.copyfile(p/'publish_integration.py',r/'scripts/publish_integration.py')
print(json.dumps({'source':g['head'],'rust':rust['totals'],'units':rust['units_run'],'lean':lean,'python':python,'provider_rust':provider,'provider_python':provider_python},indent=2))
