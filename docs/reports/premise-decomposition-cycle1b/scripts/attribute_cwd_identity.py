"""Replay exact cwd/config/import/task identities; never exclude generic hashes."""
from pathlib import Path
import json,shlex,hashlib,copy,gzip,subprocess,importlib.util
R=Path(__file__).resolve().parents[4];P=R/'private/premise-decomposition-cycle1b';O=R/'docs/reports/premise-decomposition-cycle1b/measurements/corpus';docs={s:json.loads((P/f'runs-1/corpus/{s}/resource-diagnostics.source-effects.json').read_text()) for s in ['before','after']};db=Path(json.loads((P/'runs-1/corpus/after/invocation.json').read_text())['command'][4])/'build/compile_commands.json';entries=json.loads(db.read_text());cwd={'before':str(P/'baseline-source'),'after':str(R)}
def ser(v):return json.dumps(v,ensure_ascii=False,separators=(',',':')).encode()
def sha(v):return hashlib.sha256(v).hexdigest()
def compiler(c):return {k:c[k] for k in ['program','version','target']}
def canonical_identity(x):
 v={k:x[k] for k in ['raw_source_digest','preprocessed_source_digest','configuration_digest','dialect','source_contract_digest','profile_digest','model_library','source_revision']};v['compiler']=compiler(x['compiler']);v['helpers']={k:canonical_identity(a) for k,a in sorted(x['helpers'].items())};return v
def config(identity,file,where):
 candidates=[e for e in entries if e['file'].endswith('/'+file)];assert candidates
 # pinned select_exact_unit uses the first matching file; recursive selection
 # sorts/deduplicates stably by file, retaining the same first DB record.
 e=candidates[0];words=shlex.split(e['command']);incs=[str(Path(e['file']).parent)];defs=[];i=1
 while i<len(words):
  w=words[i]
  if w in ['-I','-isystem']:
   i+=1;x=words[i];incs.append(x if Path(x).is_absolute() else str(Path(e['directory'])/x))
  elif w.startswith('-I'):
   x=w[2:];incs.append(x if Path(x).is_absolute() else str(Path(e['directory'])/x))
  elif w=='-D':
   i+=1
   if '$' not in words[i]:defs.append(words[i])
  elif w.startswith('-D') and '$' not in w[2:]:defs.append(w[2:])
  i+=1
 body=['-std='+identity['dialect'],incs,defs,'main_source_definitions',where,compiler(identity['compiler'])];return sha(ser(body)),{'entry':e,'matching_entries':len(candidates),'configuration':body}
records=[]
for index in [16,24,63]:
 b,a=docs['before']['results'][index],docs['after']['results'][index];assert b['function']==a['function'];observations=[]
 for side,r in [('before',b),('after',a)]:
  identity=r['import_identity'];file=Path(r['file']).name
  for name,x,f in [('root',identity,file)]+[(name,x,'cio_error.c') for name,x in identity['helpers'].items()]:
   actual,receipt=config(x,f,cwd[side]);assert actual==x['configuration_digest'],(index,side,name,actual,x['configuration_digest']);wrong,_=config(x,f,cwd['after' if side=='before' else 'before']);assert wrong!=actual
   observations.append({'side':side,'identity':name,'sha256':actual,'wrong_cwd_sha256':wrong,'wrong_cwd_rejected':True,**receipt})
  digest=sha(ser(canonical_identity(identity)));assert digest==r['task']['unit']['build_digest']==r['evidence']['unit']['build_digest']==r['task']['contract_sources']['qcore:boolean-field-import:v1']
 rb=copy.deepcopy(a['import_identity']);rb['configuration_digest']=b['import_identity']['configuration_digest']
 for name in rb['helpers']:rb['helpers'][name]['configuration_digest']=b['import_identity']['helpers'][name]['configuration_digest']
 assert rb==b['import_identity']
 records.append({'index':index,'before':b,'after':a,'observations':observations})
(O/'cwd-proof-input.json').write_text(json.dumps({'database':str(db),'database_sha256':sha(db.read_bytes()),'source_algorithm':'crates/qcore-c-gateway/src/boolean_source.rs:input_identity; crates/qcore-audit/src/compile_db.rs:compiler_options_compatibility; pinned_route.rs:select_exact_unit/select_recursive_units','cwd':cwd,'records':records},indent=2)+'\n');print('configuration and full import digest proofs',len(records))

# Replay task canonicalization with the existing compiled production schemas.
helper=P/'cwd-task-provenance'
command=[str(helper),str(O/'cwd-proof-input.json'),str(O/'cwd-task-proof.json')]
result=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
assert result.returncode==0,result.stderr.decode()
task_receipts=json.loads((O/'cwd-task-proof.json').read_text());assert len(task_receipts)==len(records)
restored=copy.deepcopy(docs['after'])
for row,receipt in zip(records,task_receipts):
 index=row['index'];assert receipt['index']==index
 before,after=row['before'],restored['results'][index]
 after['import_identity']=copy.deepcopy(before['import_identity'])
 old_build=before['task']['unit']['build_digest'];new_build=after['task']['unit']['build_digest']
 assert after['evidence']['unit']['build_digest']==new_build
 after['evidence']['unit']['build_digest']=old_build
 after['task']['unit']['build_digest']=old_build
 assert after['task']['contract_sources']['qcore:boolean-field-import:v1']==new_build
 after['task']['contract_sources']['qcore:boolean-field-import:v1']=old_build
 assert len(receipt['before_obligation_ids'])==len(receipt['after_obligation_ids'])==1
 old_id,new_id=receipt['before_obligation_ids'][0],receipt['after_obligation_ids'][0]
 assert list(after['bounds'])==[new_id] and list(before['bounds'])==[old_id]
 after['bounds'][old_id]=after['bounds'].pop(new_id)
 assert after['evidence']['coverage']==[new_id];after['evidence']['coverage']=[old_id]
 assert after['task']['obligations'][0]['obligation_id']==new_id
 after['task']['obligations'][0]['obligation_id']=old_id
 assert after['witness']['task_digest']==receipt['after_task_digest']
 after['witness']['task_digest']=receipt['before_task_digest']
 assert after==before,'some complete source-effect result field changed'
assert restored==docs['before'],'unrelated source-effect result changed'
residual=json.loads(gzip.decompress((O/'attributed-residual-differences.json.gz').read_bytes()))
assert len(residual['source-effects'])==29
residual['source-effects']=[]
(O/'cwd-attributed-residual-differences.json.gz').write_bytes(gzip.compress((json.dumps(residual,indent=2)+'\n').encode(),mtime=0))
receipt={'meaning':'All29 raw source-effect leaves are separately verified cwd-only derived identities, not ignored fields. Raw tasks, bounds, witnesses and source hashes remain retained; no actual witness is transported between contexts.','source_effect_records':[row['index'] for row in records],'raw_source_effect_leaves':29,'configuration_and_import_proofs':'cwd-proof-input.json','canonical_production_task_proofs':'cwd-task-proof.json','complete_source_effect_records_equal_after_exact_identity_substitution':True,'wrong_cwd_and_wrong_build_negative_controls_rejected':True,'replay_command':command,'exit':result.returncode,'helper_sha256':sha(helper.read_bytes()),'helper_source_sha256':sha((R/'docs/reports/premise-decomposition-cycle1b/scripts/cwd_task_provenance.rs').read_bytes()),'cwd_binding':'measure.py subprocess.run(cwd=REPO), where REPO is the recorded contract_inputs_root on each side'}
(O/'cwd-attribution.json').write_text(json.dumps(receipt,indent=2)+'\n')
comparison_path=O/'comparison.json';comparison=json.loads(comparison_path.read_text());comparison['checks']['complete_source_effects_equal_after_exact_cwd_attribution']=True;comparison['cwd_attribution']='cwd-attribution.json';comparison_path.write_text(json.dumps(comparison,indent=2)+'\n')
print('all29 source-effect leaves exactly attributed; remaining4contract time leaves retained')
