from pathlib import Path
import json,shlex,subprocess,hashlib,re
p=Path(__file__).parent.resolve();m=p/'inputs-mirror-1';out=p/'prepared-probes';out.mkdir(exist_ok=False)
sha=lambda x:hashlib.sha256(Path(x).read_bytes()).hexdigest()
records=[]
for route,db,units in [('corpus',m/'c-gateway-wave-pin/fluent-bit-clang-source/build/compile_commands.json',{'tinycthread.c','tinycthread_extra.c','rdkafka_sasl_cyrus.c'}),('corpus-monkey',m/'c-gateway-wave-pin/monkey/monkey-build/compile_commands.json',{'mk_iov.c'}),('system',m/'system-compile_commands.json',{'tinycthread.c','tinycthread_extra.c','rdkafka_sasl_cyrus.c','mk_iov.c'}),('slice',m/'slice-compile_commands.json',{'tinycthread.c','tinycthread_extra.c','rdkafka_sasl_cyrus.c','mk_iov.c'})]:
 rows=json.loads(db.read_text());counts={}
 for row in rows:
  unit=Path(row['file']).name
  if unit not in units:continue
  idx=counts.get(unit,0);counts[unit]=idx+1
  args=row.get('arguments') or shlex.split(row['command']);cmd=['/usr/bin/clang' if route.startswith('corpus') else '/usr/lib/llvm-21/bin/clang'];i=1
  while i<len(args):
   if args[i] in ['-o','-MF','-MT','-MQ'] :i+=2;continue
   if args[i] in ['-c','-MD','-MMD']:i+=1;continue
   cmd.append(args[i]);i+=1
  cmd+=['-E','-dD'];target=out/f'{route}-{unit}-{idx}.i'
  with target.open('wb') as f,target.with_suffix('.stderr').open('wb') as err:r=subprocess.run(cmd,cwd=row['directory'],stdout=f,stderr=err)
  text=target.read_text();lines=text.splitlines();matches=[{'line':i+1,'text':'\n'.join(lines[max(0,i-1):i+5])} for i,line in enumerate(lines) if re.search(r'\bextern\b.*\b(writev|thrd_current|mtx_destroy)\s*\(|^#define WITH_C11THREADS\b|typedef .*\b(__thrd_t|thrd_t)\s*;|thrd_success\s*=',line)]
  records.append({'route':route,'unit':unit,'configuration':idx,'database':str(db),'database_sha256':sha(db),'command':cmd,'cwd':row['directory'],'exit':r.returncode,'source':row['file'],'source_sha256':sha(row['file']),'preprocessed':str(target),'preprocessed_sha256':sha(target),'snippets':matches})
 assert set(counts)==units,(route,counts)
 assert all(v==1 for v in counts.values()),(route,counts)
(p/'prepared-resolution.json').write_text(json.dumps(records,indent=1)+'\n');print(json.dumps({'probes':len(records),'exits':[r['exit'] for r in records],'standalone_monkey_included':True}));assert all(r['exit']==0 for r in records)
