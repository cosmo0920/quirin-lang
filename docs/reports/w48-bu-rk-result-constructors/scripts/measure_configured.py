#!/usr/bin/env python3
"""Measure a held binary, retaining exact data/source identities and all exits."""
import argparse,datetime,hashlib,json,os,shutil,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
PRIMARY=Path('/home/cosmo/Documents/GitHub/qcore-c')
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('route',choices=['system','slice']);p.add_argument('out',type=Path)
p.add_argument('--binary',type=Path,required=True);p.add_argument('--binary-source',required=True)
a=p.parse_args();out=a.out.resolve();out.mkdir(parents=True,exist_ok=False)
assert shutil.disk_usage(ROOT).free>=40*1024**3,'less than 40 GiB free'
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def tree(path):return {str(x.relative_to(path)):sha(x) for x in sorted(path.rglob('*')) if x.is_file() and '.git' not in x.relative_to(path).parts}
def data():
 paths=[ROOT/'providers/contract-load-path.json',ROOT/'contract/corpus.json']
 for krate in (ROOT/'providers').iterdir():
  if krate.is_dir():
   paths.extend(x for x in krate.glob('*contracts.lock') if x.is_file())
   if (krate/'contracts').is_dir():paths.extend(x for x in (krate/'contracts').rglob('*') if x.is_file())
 paths.extend(x for x in (ROOT/'contract/profiles').rglob('*') if x.is_file())
 return {str(x.relative_to(ROOT)):sha(x) for x in sorted(paths)}
prep=PRIMARY/('private/w45-fb/w46-external-callee-contracts/attempt-1' if a.route=='system' else 'private/w47-tail/attempt-1/prep')
source=prep/'fluent-bit-clang-source';db=prep/'fluent-bit-clang-production/compile_commands.json'
selection=ROOT/'contract/profiles/fluent-bit'/('system-selection.json' if a.route=='system' else 'tail-pause-selection.json')
binary=a.binary.resolve();link=ROOT/'docs/reports/w48-bu-rk-result-constructors/link-map.corpus.json'
command=[str(binary),'current','--checkout',str(source),'--profile-directory',str(ROOT/'contract/profiles/fluent-bit'),'--compile-database',str(db),'--selection',str(selection),'--jobs','4','--report',str(out/'resource-diagnostics.json')]
env=dict(os.environ)
for k in ['QCORE_SOURCE_CONTRACTS','QCORE_RETURN_CENSUS','QCORE_CHUNKIO_PACKAGE']:env.pop(k,None)
env.update(QCORE_PROFILES=str(ROOT/'contract/profiles'),QCORE_CLANG='/usr/bin/clang',QCORE_CROSS_UNIT_LINK_MAP=str(link),QCORE_NULL_PROPERTY=str(out/(a.route+'-null.jsonl')),QCORE_OUTER_PROJECTION=str(out/(a.route+'-outer.json')))
with (out/'clang-version.txt').open('w') as f:clang=subprocess.run(['/usr/bin/clang','--version'],stdout=f,stderr=f)
(out/'clang-version.exit').write_text(str(clang.returncode)+'\n')
record={'driver_sha256':sha(Path(__file__)),'route':a.route,'command':command,'cwd':str(ROOT),'analyzer_source':a.binary_source,'controller_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'binary':str(binary),'binary_sha256':sha(binary),'data_before':data(),'source_root':str(source),'source_revision':subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip(),'database_sha256':sha(db),'selection_sha256':sha(selection),'link_map_sha256':sha(link),'jobs':4,'environment':{k:env.get(k,'unset') for k in ['QCORE_PROFILES','QCORE_CLANG','QCORE_CROSS_UNIT_LINK_MAP','QCORE_NULL_PROPERTY','QCORE_OUTER_PROJECTION','QCORE_SOURCE_CONTRACTS','QCORE_RETURN_CENSUS','QCORE_CHUNKIO_PACKAGE']},'started':datetime.datetime.now(datetime.timezone.utc).isoformat()}
source_before=tree(source);(out/'source-inputs-before.json').write_text(json.dumps(source_before,sort_keys=True)+'\n')
(out/'invocation.json').write_text(json.dumps(record,indent=2)+'\n')
with (out/'console.log').open('w') as stdout,(out/'stderr.log').open('w') as stderr:r=subprocess.run(command,cwd=ROOT,env=env,stdout=stdout,stderr=stderr)
record.update(audit_exit=r.returncode,ended=datetime.datetime.now(datetime.timezone.utc).isoformat(),binary_sha256_after=sha(binary),data_after=data(),database_sha256_after=sha(db),selection_sha256_after=sha(selection),link_map_sha256_after=sha(link))
source_after=tree(source);(out/'source-inputs-after.json').write_text(json.dumps(source_after,sort_keys=True)+'\n')
record['driver_sha256_after']=sha(Path(__file__))
stable=record['driver_sha256']==record['driver_sha256_after'] and source_before==source_after and record['data_before']==record['data_after'] and all(record[k]==record[k+'_after'] for k in ['binary_sha256','database_sha256','selection_sha256','link_map_sha256'])
record['source_files']=len(source_before);record['identities_stable']=stable
record['exit']=r.returncode if stable and clang.returncode==0 else 101
(out/'invocation.json').write_text(json.dumps(record,indent=2)+'\n');(out/'exit').write_text(str(record['exit'])+'\n')
print(a.route,record['exit'],flush=True);raise SystemExit(record['exit'])
