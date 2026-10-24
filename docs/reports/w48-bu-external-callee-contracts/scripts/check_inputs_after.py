"""Rehash every previously inventoried input; report changes without exclusions."""
from pathlib import Path
import json,hashlib,datetime,subprocess
import argparse
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--evidence-root',type=Path,default=Path(__file__).parent);parser.add_argument('--output',type=Path);args=parser.parse_args()
P=args.evidence_root.resolve()
sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
result={'recorded':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'All paths in the before manifests; additions separately enumerated for private mirror; no exclusions.'}
for name in ['native-inputs-before','host-declarations-before']:
 before=json.loads((P/(name+'.json')).read_text()); changes=[]
 for row in before['files']:
  f=Path(row['path']); actual={'exists':f.is_file()}
  if f.is_file():actual.update(bytes=f.stat().st_size,sha256=sha(f))
  if 'resolved_path'in row:actual['resolved_path']=str(f.resolve())
  if not actual['exists'] or actual['sha256']!=row['sha256'] or actual['bytes']!=row['bytes'] or ('resolved_path'in row and actual.get('resolved_path')!=row['resolved_path']):changes.append({'before':row,'after':actual})
 result[name]={'files_checked':len(before['files']),'changes':changes}
before=json.loads((P/'mirror-files-before.json').read_text())['files']; root=P/'inputs-mirror-1'
after={str(f.relative_to(root)):sha(f) for f in sorted(root.rglob('*')) if f.is_file()}
result['mirror']={'files_before':len(before),'files_after':len(after),'changes':[{'path':f,'base':before.get(f),'after':after.get(f)} for f in sorted(set(before)|set(after)) if before.get(f)!=after.get(f)]}
clang=[]
for row in json.loads((P/'analysis-clang-identities.json').read_text()):
 r=subprocess.run(row['command'],text=True,capture_output=True);f=Path(row['command'][0]);actual={'command':row['command'],'exit':r.returncode,'realpath':str(f.resolve()),'output':r.stdout,'stderr':r.stderr,'binary_sha256':sha(f)}
 clang.append({'unchanged':actual==row,'before':row,'after':actual})
result['clang']=clang
(args.output or P/'inputs-after.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'changes':{k:len(v['changes']) for k,v in result.items() if isinstance(v,dict) and 'changes'in v},'clang_unchanged':all(v['unchanged'] for v in clang)}))
