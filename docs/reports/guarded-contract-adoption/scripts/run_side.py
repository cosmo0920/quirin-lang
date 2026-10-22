#!/usr/bin/env python3
"""Run all three immutable-input native routes from one identical cwd."""
import argparse,datetime,fcntl,hashlib,json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
p=argparse.ArgumentParser();p.add_argument('side',choices=['base','tip']);p.add_argument('--freeze',type=Path,required=True);p.add_argument('--build-receipt',type=Path,required=True);p.add_argument('--run',required=True);a=p.parse_args()
out=ROOT/'private/guarded-contract-adoption'/a.run;out.mkdir(parents=True,exist_ok=False)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
receipt=json.loads(a.build_receipt.read_bytes());frozen=json.loads((a.freeze/'receipt.json').read_bytes());source=receipt['source'];assert source==frozen['source'] and receipt['sha256']==frozen['binary_sha256'];assert receipt['build']['exit']==0
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip();records=[]
def guard():
 assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()==head
 assert not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True)
 assert sha(a.build_receipt)==build_sha and sha(a.freeze/'receipt.json')==freeze_sha
 for name,digest in frozen['inputs'].items():assert sha(a.freeze/'inputs'/name)==digest,name
build_sha=sha(a.build_receipt);freeze_sha=sha(a.freeze/'receipt.json')
with open('/tmp/qcore-c-card-validation-20261022.lock','a') as lock:
 print('WAITING FOR NATIVE',a.side,'LOCK',flush=True);fcntl.flock(lock,fcntl.LOCK_EX);guard()
 for route in ['system','slice','corpus']:
  command=[sys.executable,str(ROOT/'docs/reports/guarded-contract-adoption/scripts/measure.py'),route,str(out/route),'--link-map',str(ROOT/'docs/reports/w48-bottom-up-batch4/link-map.corpus.json'),'--inputs-snapshot',str(a.freeze/'inputs'),'--binary-source',source,'--jobs','4']
  start=datetime.datetime.now(datetime.timezone.utc).isoformat();print('START',a.side,route,start,flush=True)
  with (out/(route+'.stdout.log')).open('wb') as stdout,(out/(route+'.stderr.log')).open('wb') as stderr:r=subprocess.run(command,cwd=ROOT,stdout=stdout,stderr=stderr)
  records.append({'route':route,'command':command,'cwd':str(ROOT),'exit':r.returncode,'started':start,'ended':datetime.datetime.now(datetime.timezone.utc).isoformat()});(out/'commands.json').write_text(json.dumps(records,indent=2)+'\n');guard();print('END',a.side,route,r.returncode,flush=True);assert r.returncode==0
 (out/'completion.json').write_text(json.dumps({'side':a.side,'exit':0,'reporting_head':head,'analyzer_source':source,'build_receipt':str(a.build_receipt),'build_receipt_sha256':build_sha,'freeze_receipt_sha256':freeze_sha,'commands_sha256':sha(out/'commands.json'),'completed':datetime.datetime.now(datetime.timezone.utc).isoformat()},indent=2)+'\n')
print('COMPLETE',a.side,flush=True)
