"""Recompute only cwd-dependent provenance identities; never suppress effect payloads."""
import copy,hashlib,json

def sha(text):return hashlib.sha256(text.encode()).hexdigest()
def canonical(x):return json.dumps(x,ensure_ascii=False,separators=(',',':'))
def rust_debug_string(text):
 assert text.startswith('"')
 out=[];i=1;escapes={'n':'\n','r':'\r','t':'\t','0':'\0','\\':'\\','"':'"',"'":"'"}
 while i<len(text):
  c=text[i];i+=1
  if c=='"':return ''.join(out),i
  if c!='\\':out.append(c);continue
  c=text[i];i+=1
  if c in escapes:out.append(escapes[c])
  elif c=='u' and text[i]=='{':
   end=text.index('}',i);out.append(chr(int(text[i+1:end],16)));i=end+1
  elif c=='x':out.append(chr(int(text[i:i+2],16)));i+=2
  else:raise AssertionError('Unknown Rust debug escape: '+c)
 raise AssertionError('Unterminated Rust debug string')

class Provenance:
 def __init__(self,records,tasks):
  self.builds=[{},{}];self.configs=[{},{}];self.tasks={r['task_digest']:r for r in tasks}
  self.records=records;compiler=records['compiler'];before=records['before_cwd'];after=records['after_cwd']
  expected='ClangIdentity { '+', '.join(k+': '+canonical(compiler[k]) for k in ['program','version','target'])+' }'
  for r in records['rows']:
   assert r['compiler_debug']==expected
   prefix='C11TranslationUnit { source: ';assert r['unit_debug'].startswith(prefix)
   source,end=rust_debug_string(r['unit_debug'][len(prefix):]);assert sha(source)==r['source_sha256']
   assert r['unit_debug'][len(prefix)+end:].startswith(', include_paths: ')
   common=sha(r['unit_debug']+'\n'+r['compiler_debug'])
   configs=[]
   for i,(side,cwd) in enumerate([('before',before),('after',after)]):
    raw='('+r['unit_debug']+', '+canonical(cwd)+', '+r['compiler_debug']+')';assert sha(raw)==r[side+'_build']
    self.builds[i][r[side+'_build']]=common
    text=r[side+'_configuration_bytes'];assert sha(text)==r[side+'_configuration']
    value,end=json.JSONDecoder().raw_decode(text);assert len(value)==6 and value[4]==cwd and value[5]==compiler
    value[4]='<recorded-corpus-working-directory>'
    configs.append(canonical(value)+text[end:])
   assert configs[0]==configs[1]
   cfg=sha(configs[0])
   for i,side in enumerate(['before','after']):
    old=self.configs[i].setdefault(r[side+'_configuration'],cfg);assert old==cfg
 def context(self,value,side,counts):
  old=value['build_identity']
  if old in self.builds[side]:
   value['build_identity']=self.builds[side][old];counts['independently_recomputed_cwd_builds']+=1
 def identity(self,value,side,normalized=False):
  keys='raw_source_digest preprocessed_source_digest configuration_digest dialect source_contract_digest profile_digest model_library source_revision compiler helpers'.split();assert set(value)==set(keys)
  out={k:copy.deepcopy(value[k]) for k in keys}
  out['compiler']={k:value['compiler'][k] for k in ['program','version','target']};assert set(value['compiler'])==set(out['compiler'])
  out['helpers']={k:self.identity(v,side,normalized) for k,v in sorted(value['helpers'].items())}
  if normalized:out['configuration_digest']=self.configs[side][out['configuration_digest']]
  return out
 def effects(self,value,side,counts):
  out=copy.deepcopy(value)
  for row in out.get('results',[]):
   if 'task' not in row:continue
   original=copy.deepcopy(row);task=row['task'];digest=row['witness']['task_digest'];record=self.tasks[digest]
   assert json.loads(record['task_canonical_json'])==task and sha(record['task_canonical_json'])==digest
   identity=self.identity(row['import_identity'],side);build=sha(canonical(identity))
   assert build==task['unit']['build_digest']==task['contract_sources']['qcore:boolean-field-import:v1']
   norm_identity=self.identity(identity,side,True);norm_build=sha(canonical(norm_identity));newids={}
   assert len(task['obligations'])==len(record['obligations'])
   for obligation,proof in zip(task['obligations'],record['obligations']):
    subject=obligation['subject'];assert subject['subject']=='function'
    fields=[str(task['task_version']),task['unit']['source_digest'],build,task['unit']['translation_unit'],task['semantic_profile'],obligation['capability'],obligation['property'],'function|'+subject['translation_unit']+'|'+subject['symbol']]
    preimage='\x1f'.join(fields);assert preimage==proof['preimage']
    identifier=obligation['capability']+':'+sha(preimage)[:24];assert identifier==proof['id']==obligation['obligation_id']
    fields[2]=norm_build;newids[identifier]=obligation['capability']+':'+sha('\x1f'.join(fields))[:24]
   assert row['evidence']['unit']==task['unit']
   assert set(row['evidence']['coverage'])==set(newids)==set(row['bounds'])
   for o in task['obligations']:assert o['bounds']==task['bounds']==row['bounds'][o['obligation_id']]
   task['unit']['build_digest']=norm_build;task['contract_sources']['qcore:boolean-field-import:v1']=norm_build
   row['evidence']['unit']['build_digest']=norm_build
   for o in task['obligations']:o['obligation_id']=newids[o['obligation_id']]
   row['bounds']={newids[k]:v for k,v in row['bounds'].items()}
   row['evidence']['coverage']=sorted(newids[k] for k in row['evidence']['coverage'])
   row['import_identity']=norm_identity;row['witness']['task_digest']='<independently-recomputed-cwd-dependent-task>'
   counts['independently_recomputed_source_effect_tasks']+=1
  return out
