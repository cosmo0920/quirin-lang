#!/usr/bin/env python3
"""Exact report controls at one cwd/input identity; production runs are separate.

CFG-RELEASE copy of docs/reports/audit-ir-retained-products/scripts/parity.py; only the
recorded fixed build label differs (BASE 710021bd7)."""
import argparse,hashlib,json,re,shutil,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--before',type=Path,required=True);p.add_argument('--after',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
root=a.output.resolve();root.mkdir(parents=True,exist_ok=False);checkout=root/'checkout';(checkout/'src').mkdir(parents=True)
fixture=Path('crates/qcore-c-gateway/tests/refuted_contract_withdrawal.rs').read_text()
raw=lambda name: re.search(r'const '+name+r': &str = r#"(.*?)"#;',fixture,re.S).group(1)
prelude='typedef unsigned long size_t;\n'+re.search(r'const PRELUDE:.*?r#"(.*?)"#',fixture,re.S).group(1)
sources={'a_bodies.c':prelude+raw('BODIES'),'b_callers.c':prelude+raw('CALLERS'),'c_safe.c':'void safe_body(void) { return; }\n','d_data.c':'typedef int no_function;\n'}
for name,text in sources.items():(checkout/'src'/name).write_text(text)
def run(argv,cwd=checkout):return subprocess.check_output(argv,cwd=cwd,text=True).strip()
run(['git','init','--quiet']);run(['git','add','src']);run(['git','-c','user.name=Retained products control','-c','user.email=control@example.invalid','commit','--quiet','-m','Fixed control inputs'])
head=run(['git','rev-parse','HEAD'])
(root/'contracts.qv').write_text(raw('CONTRACTS').replace('requires resource_contract_v1;', 'requires resource_contract_v1; requires resource_revision(flexible);'))
(root/'contracts.json').write_text(json.dumps({'schema_version':1,'modules':['contracts.qv']}))
profile=root/'sample';profile.mkdir()
library={'pin':'sample','source_roots':[{'root':'src','selection':'top_level'}],'audited_files':[{'path':'src/'+name,'sha256':hashlib.sha256(text.encode()).hexdigest()} for name,text in sources.items()],'source_witnesses':[],'permitted_mechanisms':[],'operation_summaries':{},'api_dispositions':{},'accepted_builds':[],'required_evidence':[],'typed_import':None,'callback_summaries':{},'boolean_field_models':[],'provider':None}
(profile/'profile.json').write_text(json.dumps({'schema_version':1,'source_set':'sample','pins':{'sample':{'git_commit':head,'clean_paths':[],'description':'fixed parity control'}},'libraries':{'sample':library},'resource_contract_manifest':'../contracts.json','provider_manifests':[]}))
(root/'compile_commands.json').write_text(json.dumps([{'directory':str(checkout),'file':'src/'+name,'arguments':['clang','-std=c11','-c','src/'+name,'-o',name+'.o']} for name in sources]))
config={'schema_version':1,'profile_directory':str(profile),'checkout':str(checkout),'compile_database':str(root/'compile_commands.json'),'libraries':['sample'],'report':str(root/'published/report.json')}
(root/'audit.json').write_text(json.dumps(config))
records=[]
for label,binary,jobs in [('before',a.before,1),('after1',a.after,1),('after2',a.after,2)]:
 if (root/'published').exists():shutil.rmtree(root/'published')
 argv=[str(binary.resolve()),str(root/'audit.json'),'--jobs',str(jobs)]
 result=subprocess.run(argv,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 out=root/label;out.mkdir();(out/'stdout').write_bytes(result.stdout);(out/'stderr').write_bytes(result.stderr)
 if (root/'published').exists():shutil.copytree(root/'published',out/'published')
 records.append({'label':label,'argv':argv,'exit':result.returncode,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest()})
 if result.returncode:raise RuntimeError(records[-1])
before=root/'before/published';comparisons=[]
for label in ['after1','after2']:
 files=sorted(p.name for p in before.iterdir());assert files==sorted(p.name for p in (root/label/'published').iterdir())
 for name in files:
  left=(before/name).read_bytes();right=(root/label/'published'/name).read_bytes();equal=left==right
  scheduling_only = False
  if name == 'report.invocation.json' and label == 'after2':
   l=json.loads(left);r=json.loads(right)
   assert l.pop('jobs') == {'effective':1,'requested':1,'source':'command_line'}
   assert r.pop('jobs') == {'effective':2,'requested':2,'source':'command_line'}
   assert l['memory_admission'].pop('phases') == []
   phases=r['memory_admission'].pop('phases')
   assert len(phases)==2 and all(x['tasks']==4 and x['requested_maximum']==2 and 1<=x['admitted']<=2 for x in phases)
   scheduling_only=l==r
  comparisons.append({'scheduling_only':scheduling_only,'after':label,'file':name,'byte_equal':equal,'before_sha256':hashlib.sha256(left).hexdigest(),'after_sha256':hashlib.sha256(right).hexdigest()})
report=json.loads((before/'report.json').read_text());sidecar=json.loads((before/'report.assumptions.json').read_text())
receipt={'after_source':run(['git','rev-parse','HEAD'],Path.cwd()),'fixed_build_identity':'710021bd713b90037e803d4a1f87d95eba6476c9','runs':records,'comparisons':comparisons,'units':len(report['units']),'functions':sum(len(u.get('functions',[])) for u in report['units']),'refutations':sidecar.get('refuted_contracts'),'scope':'Same cwd and exact inputs; synthetic declared counted-user and custody refutations, cross-unit callers, Safe certificate replay, and data-only units. All report/assumptions bytes must match. Invocation metadata at jobs=2 may differ only in explicitly checked jobs and two phase observations; raw hashes retained. No timing normalization. Candidate built with QCORE_AUDIT_BUILD_COMMIT fixed to base for this control only.'}
(root/'parity.json').write_text(json.dumps(receipt,indent=2)+'\n')
assert all(x['byte_equal'] or x['scheduling_only'] for x in comparisons),receipt
assert sidecar['refuted_contracts']['phases']==2,sidecar['refuted_contracts']
print(json.dumps(receipt,indent=2))
