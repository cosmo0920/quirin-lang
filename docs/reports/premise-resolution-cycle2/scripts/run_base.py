import concurrent.futures,datetime,fcntl,json,pathlib,subprocess,sys
root=pathlib.Path.cwd();out=root/'private/premise-resolution-cycle2/base-collector-1';out.mkdir(exist_ok=False);records=[]
def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def run(route,jobs):
 target=root/'private/premise-resolution-cycle2/measure-1'/route/'base'
 cmd=[sys.executable,'docs/reports/premise-resolution-cycle2/scripts/measure.py',route,str(target),'--link-map','docs/reports/premise-resolution-cycle2/link-map.corpus.json','--inputs-snapshot','private/premise-resolution-cycle2/base-freeze/inputs','--binary-source','c1625baf713e4d8498c39282238675749237005d','--jobs',str(jobs)]
 start=now()
 with (out/(route+'.log')).open('w') as log:p=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
 r=dict(route=route,command=cmd,started=start,ended=now(),exit=p.returncode,log=str(out/(route+'.log')));print(json.dumps(r),flush=True);return r
with open('/tmp/qcore-c-card-validation-20261021.lock','a') as lock:
 fcntl.flock(lock,fcntl.LOCK_EX);(out/'acquired.json').write_text(json.dumps({'utc':now(),'head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()},indent=2)+'\n');print('BASE LOCK ACQUIRED',now(),flush=True)
 with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:records.extend(pool.map(lambda r:run(r,8),['system','slice']))
 (out/'commands.json').write_text(json.dumps(records,indent=2)+'\n')
 if any(r['exit'] for r in records):raise SystemExit(1)
 records.append(run('corpus',16));(out/'commands.json').write_text(json.dumps(records,indent=2)+'\n')
 if records[-1]['exit']:raise SystemExit(1)
(out/'released.json').write_text(json.dumps({'utc':now()},indent=2)+'\n');print('BASE COMPLETE',now(),flush=True)
