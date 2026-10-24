from pathlib import Path
import datetime,json,os,shutil,subprocess,sys,time
p=Path('private/w48-cmt-member-storage');slot=json.loads((p/'development-lock.json').read_text());assert slot['state']=='development_slot_acquired'
ancestor=os.getppid()
while ancestor>1 and ancestor!=slot['pid']:
    ancestor=int(Path(f'/proc/{ancestor}/stat').read_text().rsplit(')',1)[1].split()[1])
assert ancestor==slot['pid'],'heavy checks must descend from the held slot'
assert shutil.disk_usage(Path.cwd()).free>=40*1024**3
name=sys.argv[1];command=sys.argv[2:];out=p/'steps';out.mkdir(exist_ok=True);log=out/(name+'.log');assert not log.exists()
r={'command':command,'started':datetime.datetime.now(datetime.timezone.utc).isoformat(),'head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'dirty':bool(subprocess.check_output(['git','status','--porcelain'],text=True).strip()),'log':str(log),'slot_pid':slot['pid']};started=time.monotonic()
with log.open('wb') as stream:code=subprocess.run(command,stdout=stream,stderr=stream).returncode
r.update(exit=code,seconds=time.monotonic()-started,ended=datetime.datetime.now(datetime.timezone.utc).isoformat());(out/(name+'.json')).write_text(json.dumps(r,indent=2)+'\n');(out/(name+'.exit')).write_text(str(code)+'\n');print(json.dumps(r,indent=2),flush=True)
raise SystemExit(code)
