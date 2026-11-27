from pathlib import Path
import subprocess,datetime,json,hashlib,sys
p=Path('private/cmt-disjoint-frames');assert (p/'tip-campaign.exit').read_text().strip()=='0'
out=p/'attribution-1';out.mkdir(exist_ok=False)
record={'steps':[],'started':datetime.datetime.now(datetime.timezone.utc).isoformat()}
for name,args in [('retain-slice',['retain_compare.py','slice']),('replay',['post_attribution.py','replay-v3']),*[(r+'-attribution',['attribute_compare.py',r]) for r in ['corpus','system','slice']],('review-unchanged',['review_unchanged.py']),('publish',['publish_unchanged.py']),('amendments',['prepare_amendments_unchanged.py'])]:
 cmd=['python3',str(p/args[0]),*args[1:]];step={'name':name,'command':cmd,'started':datetime.datetime.now(datetime.timezone.utc).isoformat(),'log':str(out/(name+'.log')),'script_sha256':hashlib.sha256(Path(cmd[1]).read_bytes()).hexdigest()}
 with Path(step['log']).open('wb') as f:code=subprocess.run(cmd,stdout=f,stderr=f).returncode
 step.update(exit=code,ended=datetime.datetime.now(datetime.timezone.utc).isoformat(),log_sha256=hashlib.sha256(Path(step['log']).read_bytes()).hexdigest());record['steps'].append(step);(out/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');print(name,code,flush=True)
 if code:sys.exit(code)
record.update(exit=0,ended=datetime.datetime.now(datetime.timezone.utc).isoformat());(out/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');(out/'exit').write_text('0\n')
