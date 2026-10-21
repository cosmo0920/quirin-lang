"""Official reviewed-source pin maintenance; retain every command and complete identity."""
import argparse,copy,datetime,fcntl,hashlib,json,os,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];REPORT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('output',type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);inv=json.loads((REPORT/'inventory-before.json').read_bytes());binary=ROOT/'target/debug/quirin-audit';env=dict(os.environ,CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0');records=[]
primary=Path('/home/cosmo/Documents/GitHub/qcore-c');sources=[primary/'build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source',primary/'private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source'];heads={str(s):subprocess.check_output(['git','-C',str(s),'rev-parse','HEAD'],text=True).strip() for s in sources}
reviewed={r['revision'] for e in inv['pin_entries'] if e['mode']=='pinned' for ident in e['before']['identities'] for r in ident['reviewed_at'] if r['kind']=='git'};assert reviewed==set(heads.values()),(reviewed,heads)
(out/'source-roots.json').write_text(json.dumps({'actual_source_heads':heads,'exact_original_reviewed_revision_set':sorted(reviewed),'pin_binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'review_scope':'The frozen whole8module originals are recovered exactly after the selected metadata lines are removed; exhaustive Rust validates complete interpreted/graph/binding equality. Existing reviewed C identities must be reproduced exactly by official CLI; no new C-body claim is introduced.'},indent=2)+'\n')
def run(name,cmd,allowed=(0,)):
 with (out/(name+'.log')).open('w') as log:r=subprocess.run(list(map(str,cmd)),cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
 records.append({'name':name,'command':list(map(str,cmd)),'cwd':str(ROOT),'exit':r.returncode,'ended':datetime.datetime.now(datetime.timezone.utc).isoformat(),'log':str(out/(name+'.log'))});(out/'commands.json').write_text(json.dumps(records,indent=2)+'\n');print(name,r.returncode,flush=True);assert r.returncode in allowed
with open('/tmp/qcore-c-cycle2-small-compile.lock','a') as lock:
 fcntl.flock(lock,fcntl.LOCK_EX);(out/'acquired.json').write_text(json.dumps({'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()}))
 # Official sync must first publish fresh module/load-path/inventory hashes.
 # Its lock-verification refusal is expected before official repinning, and is
 # retained as a refusal, never represented as a green check or waived.
 run('sync-before-pin',[sys.executable,'providers/contracts.py','sync'],allowed=(0,1))
 if records[-1]['exit']:assert 'contract locks are stale or unreadable' in (out/'sync-before-pin.log').read_text()
 groups={}
 for e in inv['pin_entries']:groups.setdefault((e['module'].split('/')[1],e['view'],e['mode']),[]).append(e)
 for i,((crate,view,mode),entries) in enumerate(sorted(groups.items())):
  for j,source in enumerate(sources if mode=='pinned' else sources[:1]):
   command=[binary,'pin','--source-root',source,'--crate',crate]
   if view=='bounds':command.append('--bounds')
   for e in sorted(entries,key=lambda e:e['declaration']):command+=['--function',e['declaration']]
   run(f'pin-{i}-{j}',command)
 checked=[]
 for lockfile in sorted({e['lock'] for e in inv['pin_entries']}):
  before=json.loads(subprocess.check_output(['git','show',inv['base']+':'+lockfile],cwd=ROOT));after=json.loads((ROOT/lockfile).read_bytes());restored=copy.deepcopy(after)
  for e in [e for e in inv['pin_entries'] if e['lock']==lockfile]:
   old=e['before'];new=next(r for r in after[e['mode']] if r['function']==e['declaration']);same=dict(new);same['contract_sha256']=old['contract_sha256'];assert same==old,('source/body/revision/mode changed',lockfile,e['declaration']);assert new['contract_sha256']!=old['contract_sha256']
   actual=next(r for r in restored[e['mode']] if r['function']==e['declaration']);actual['contract_sha256']=old['contract_sha256'];checked.append({'lock':lockfile,'function':e['declaration'],'mode':e['mode'],'before_contract_sha256':old['contract_sha256'],'after_contract_sha256':new['contract_sha256'],'complete_original_identity_exact':True})
  assert restored==before,('unrelated lock field changed',lockfile)
 (out/'identities.json').write_text(json.dumps({'count':len(checked),'pinned':166,'flexible':2,'complete_lock_objects_equal_after_only_selected_contract_text_sha_restoration':True,'selected':checked},indent=2)+'\n')
 run('sync-after-pin',[sys.executable,'providers/contracts.py','sync']);run('check-after-pin',[sys.executable,'providers/contracts.py','check'])
(out/'released.json').write_text(json.dumps({'utc':datetime.datetime.now(datetime.timezone.utc).isoformat()}));print('official source pins/inventories complete0',flush=True)
