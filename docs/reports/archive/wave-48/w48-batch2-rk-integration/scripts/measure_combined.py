#!/usr/bin/env python3
"""Fresh combined W48/RK measurements; run with REPO BUILT_BINARY OUTPUT_DIR."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,sys
repo,built,out=map(lambda x:Path(x).resolve(),sys.argv[1:4])
if out.exists(): raise SystemExit('output directory already exists')
out.mkdir(parents=True)
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip()
binary=repo/'target/release/quirin-audit'
binary.parent.mkdir(parents=True,exist_ok=True)
if binary.resolve()!=built: shutil.copyfile(built,binary)
binary.chmod(binary.stat().st_mode | 0o111)
digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
link=repo/'docs/reports/w48-bottom-up-batch2/link-map.corpus.json'
env=os.environ.copy();env.pop('QCORE_TYPED_PATH_PARITY',None);env.pop('QCORE_BOUNDS_PROPERTY',None);env.update(QCORE_CROSS_UNIT_LINK_MAP=str(link),QCORE_CLANG='/usr/lib/llvm-21/bin/clang')
(out/'head').write_text(head+'\n')
(out/'binary.sha256').write_text(digest(binary)+'\n')
(out/'link-map.sha256').write_text(digest(link)+'\n')
prepared={
 'system':(repo/'private/w45-fb/w46-external-callee-contracts/attempt-1','system-selection.json'),
 'slice':(repo/'private/w47-tail/attempt-1/prep','tail-pause-selection.json'),
}
for route in ['corpus','system','slice']:
 env['QCORE_NULL_PROPERTY']=str(out/f'{route}-null.jsonl')
 env['QCORE_OUTER_PROJECTION']=str(out/f'{route}-outer.json')
 if route=='corpus':
  command=[sys.executable,str(repo/'scripts/run_pinned_audit.py'),str(out/route),'66910c10','--inputs-root',str(repo/'build/cycle19'),'--jobs','4']
  context={'source_revision':'66910c10','inputs_root':str(repo/'build/cycle19')}
 else:
  prep,name=prepared[route];checkout=prep/'fluent-bit-clang-source';database=prep/'fluent-bit-clang-production/compile_commands.json';selection=repo/'contract/profiles/fluent-bit'/name
  (out/route).mkdir()
  command=[str(binary),'current','--checkout',str(checkout),'--profile-directory',str(repo/'contract/profiles/fluent-bit'),'--compile-database',str(database),'--selection',str(selection),'--jobs','4','--report',str(out/route/'resource-diagnostics.json')]
  context={'source_revision':subprocess.check_output(['git','rev-parse','HEAD'],cwd=checkout,text=True).strip(),'checkout':str(checkout),'compile_database':str(database),'compile_database_sha256':digest(database),'selection':str(selection),'selection_sha256':digest(selection)}
 record={'head':head,'binary':str(binary),'binary_sha256':digest(binary),'command':command,'jobs':4,'link_map':str(link),'link_map_sha256':digest(link),'environment':{key:env[key] for key in ['QCORE_CROSS_UNIT_LINK_MAP','QCORE_CLANG','QCORE_NULL_PROPERTY','QCORE_OUTER_PROJECTION']},**context}
 (out/f'{route}-invocation.json').write_text(json.dumps(record,indent=2)+'\n')
 with (out/f'{route}.log').open('w') as stdout,(out/f'{route}.stderr').open('w') as stderr:
  code=subprocess.run(command,cwd=repo,env=env,stdout=stdout,stderr=stderr).returncode
 (out/f'{route}.exit').write_text(str(code)+'\n');print(route,code,flush=True)
 if code: raise SystemExit(code)
 if subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip()!=head: raise SystemExit('source HEAD changed during measurement')
 if digest(binary)!=record['binary_sha256']: raise SystemExit('binary changed during measurement')
