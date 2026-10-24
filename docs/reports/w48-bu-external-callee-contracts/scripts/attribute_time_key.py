"""Reconstruct the exact empty Monkey lifetime-summary source/config keys, including DATE/TIME."""
from pathlib import Path
import argparse,json,hashlib,gzip,subprocess,shlex,re,datetime
ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--evidence-root',type=Path,default=Path(__file__).parent);ap.add_argument('--repo',type=Path,default=Path.cwd());ap.add_argument('--replay-output-root',type=Path);ap.add_argument('--routes',nargs='+',default=['corpus','system','slice']);a=ap.parse_args();P=a.evidence_root.resolve();output_root=(a.replay_output_root or P).resolve();output_root.mkdir(parents=True,exist_ok=True);read=lambda f:json.loads(f.read_bytes());sha=lambda b:hashlib.sha256(b).hexdigest()
paths=[['callback_members','lifetime_protocols','evidence','summaries'],['callback_members','prepass','lifetime_protocols','summaries']]
def at(v,path):
 for k in path:v=v[k]
 return v
for route in a.routes:
 dirs={s:P/('base-3'if s=='base'else'tip-1')/route for s in ['base','tip']};invs={s:read(d/'invocation.json')for s,d in dirs.items()};assert all(v.get('exit')==0 for v in invs.values())
 docs={s:read(d/('corpus/resource-diagnostics.contracts.json'if route=='corpus'else'resource-diagnostics.contracts.json'))for s,d in dirs.items()}
 moves=[]
 for path in paths:
  b,t=[at(docs[s],path) for s in ['base','tip']];removed=set(b)-set(t);added=set(t)-set(b)
  assert all(b[k]==t[k] for k in set(b)&set(t))
  if not removed and not added:continue
  assert len(removed)==len(added)==1;bh,th=next(iter(removed)),next(iter(added));assert b[bh]==t[th]=={};moves.append({'path':path,'base':bh,'tip':th,'payload':{}})
 if not moves:
  (output_root/(route+'-time-key-attribution.json')).write_text(json.dumps({'route':route,'exit':0,'moves':[]},indent=2)+'\n');continue
 assert len(moves)==2 and len({(r['base'],r['tip'])for r in moves})==1
 if route=='corpus':
  runs={s:read(d/'corpus/run.json') for s,d in dirs.items()};assert all(r['exit']==0 for r in runs.values());roots={s:r['command'][4]for s,r in runs.items()};assert len(set(roots.values()))==1;db=Path(roots['tip'])/'build/compile_commands.json'
 else:assert invs['base']['compile_database']==invs['tip']['compile_database'];db=Path(invs['tip']['compile_database'])
 rawdb=db.read_bytes();before=read(P/'mirror-files-before.json')['files'];key=str(db.relative_to(P/'inputs-mirror-1'));assert before[key]==sha(rawdb)
 if route!='corpus':assert all(v['compile_database_sha256']==sha(rawdb)for v in invs.values())
 entries=[e for e in json.loads(rawdb)if e['file'].endswith('/lib/monkey/mk_bin/monkey.c')];assert len(entries)==1;entry=entries[0];words=shlex.split(entry['command']);command=[words[0]];skip=False
 for w in words[1:]:
  if skip:skip=False;continue
  if w in ['-c','-MD','-MMD','-MP','-M','-MM','-MG']:continue
  if w in ['-o','-MF','-MT','-MQ']:skip=True;continue
  if (w.startswith('-o')and len(w)>2)or any(w.startswith(f)and len(w)>3 for f in ['-MF','-MT','-MQ']):continue
  command.append(w)
 command+=['-E','-P'];assert '-fPIE'in command and '-std=gnu17'in command
 out=output_root/(route+'-time-preimage');out.mkdir(exist_ok=False)
 r=subprocess.run(command,cwd=entry['directory'],stdout=subprocess.PIPE,stderr=subprocess.PIPE);(out/'stderr.log').write_bytes(r.stderr);assert r.returncode==0
 s=r.stdout;(out/'fresh-source.c.gz').write_bytes(gzip.compress(s,mtime=0));m=re.search(rb'(static const char MONKEY_BUILT\[\] = ")([^"]+)(" " " ")([^"]+)(";)',s);assert m
 prefix=b'AnonymousText\0Gnu17\0[]\0[]\0[]\0';tz=datetime.timezone(datetime.timedelta(hours=9));months='Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec'.split();matches={}
 for side in ['base','tip']:
  target=moves[0][side];inv=invs[side]
  for sec in range(int(datetime.datetime.fromisoformat(inv['started']).timestamp()),int(datetime.datetime.fromisoformat(inv['ended']).timestamp())+1):
   dt=datetime.datetime.fromtimestamp(sec,tz);date=f'{months[dt.month-1]} {dt.day:2d} {dt.year:04d}'.encode();time=dt.strftime('%H:%M:%S').encode();source=s[:m.start(2)]+date+s[m.end(2):m.start(4)]+time+s[m.end(4):]
   if sha(prefix+source)==target:
    matches[side]={'key':target,'utc':datetime.datetime.fromtimestamp(sec,datetime.timezone.utc).isoformat(),'date_literal':date.decode(),'time_literal':time.decode(),'source_sha256':sha(source)};(out/(side+'-reconstructed-source.c.gz')).write_bytes(gzip.compress(source,mtime=0));break
  assert side in matches,(side,'no complete source/config preimage for actual native key')
 producers={}
 for f in ['crates/qcore-c-gateway/src/loop_lifetime_source.rs','crates/qcore-audit/src/member_prepass.rs','crates/qcore-audit/src/compile_db.rs']:
  raws=[subprocess.check_output(['git','show',invs[side]['analyzer_source']+':'+f],cwd=a.repo)for side in ['base','tip']];assert raws[0]==raws[1];producers[f]=sha(raws[0])
 result={'route':route,'exit':0,'moves':moves,'complete_descriptor_matches':matches,'descriptor_prefix_hex':prefix.hex(),'timezone':'Asia/Tokyo +09:00','preprocess':{'command':command,'cwd':entry['directory'],'exit':r.returncode,'compile_database':str(db),'compile_database_sha256':sha(rawdb),'source_file_sha256':sha(Path(entry['file']).read_bytes()),'fresh_preprocessed_sha256':sha(s),'preprocessed_bytes':len(s)},'unchanged_producers':producers,'meaning':'Exactly one empty-summary key remap, duplicated in evidence/prepass. Both complete actual source/config keys reproduced by changing only MONKEY_BUILT DATE/TIME literals within the recorded native windows. Original artifacts and all fields retained; no C truth or semantic-summary transport claimed.'}
 (output_root/(route+'-time-key-attribution.json')).write_text(json.dumps(result,indent=2)+'\n');print({'route':route,'exit':0,'matches':matches})
