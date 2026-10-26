from pathlib import Path
import subprocess,datetime,json,hashlib,sys
p=Path('private/cmt-disjoint-frames/publication-1');p.mkdir(exist_ok=False);a='docs/reports/cmt-self-init-disjoint-frames/integration-amendments.json'
steps=[('amendments-check',['python3','scripts/apply_amendments.py',a,'--check']),('amendments-dry-run',['python3','scripts/apply_amendments.py',a,'--dry-run']),('amendments-apply',['python3','scripts/apply_amendments.py',a]),('matrix',['python3','docs/reports/remaining-d02/build_matrix.py','.','--gate','docs/reports/cmt-self-init-disjoint-frames/gate/gate.json']),('index',['python3','scripts/build_docs_index.py'])]
r={'steps':[]}
for name,cmd in steps:
 f=p/(name+'.log');s={'name':name,'command':cmd,'started':datetime.datetime.now(datetime.timezone.utc).isoformat(),'log':str(f)}
 with f.open('wb') as out:c=subprocess.run(cmd,stdout=out,stderr=out).returncode
 s.update(exit=c,ended=datetime.datetime.now(datetime.timezone.utc).isoformat(),log_sha256=hashlib.sha256(f.read_bytes()).hexdigest());r['steps'].append(s);(p/'receipt.json').write_text(json.dumps(r,indent=2)+'\n');print(name,c,flush=True)
 if c:sys.exit(c)
r['exit']=0;(p/'receipt.json').write_text(json.dumps(r,indent=2)+'\n')
