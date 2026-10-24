from pathlib import Path
import subprocess,json,os,sys,datetime,hashlib
root=Path.cwd();p=root/'private/w48-cmt-member-storage';out=p/sys.argv[1];out.mkdir(exist_ok=False)
def run(name,cmd,env=None):
 log=out/(name+'.log');r={'command':list(map(str,cmd)),'started':datetime.datetime.now(datetime.timezone.utc).isoformat(),'log':str(log)}
 with log.open('wb') as f:code=subprocess.run(list(map(str,cmd)),stdout=f,stderr=f,env=env).returncode
 r.update(exit=code,ended=datetime.datetime.now(datetime.timezone.utc).isoformat());(out/(name+'.json')).write_text(json.dumps(r,indent=2)+'\n');print(name,code,flush=True)
 if code:raise SystemExit(code)
for source,name,crates in [('rebind_readings.rs','rebind-readings',['qcore_c_gateway','qcore_contract','qcore_schemas','serde_json']),('projection_replay.rs','projection-replay',['qcore_audit','qcore_c_gateway','qcore_schemas','serde_json'])]:
 run(name+'-build',['python3','docs/reports/plugin-premise-decomposition/scripts/build_replay.py',p/source,out/name,'--binary','target/release/quirin-audit','--extern',*crates])
env=dict(os.environ,QCORE_PROFILES=str(p/'inputs/contract/profiles'),QCORE_CROSS_UNIT_LINK_MAP=str(root/'docs/reports/detach-result-wrapper-composition/link-map.corpus.json'))
(out/'time').mkdir()
for route in ['corpus','system','slice']:
 b=p/('base-'+route);t=p/('tip-'+route)
 run(route+'-readings',[out/'rebind-readings',b/'resource-diagnostics.json',t/'resource-diagnostics.json',b/'resource-diagnostics.contracts.json',t/'resource-diagnostics.contracts.json','359f64a987b74b21852c5a691e5c23f75c15d570','99457f141a90d7bc597516eda904dcf1821ada69',out/(route+'-readings-proof.json')])
 (out/'time'/route).mkdir()
 for side in ['base','tip']:(out/'time'/route/side).symlink_to(p/'comparisons'/route/side,target_is_directory=True)
 run(route+'-time',['python3',p/'time_identity.py',route,'--artifacts-root',out/'time'])
 for side,directory in [('base',b),('tip',t)]:
  if route=='corpus':
   source=Path('/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source');database=source/'build/compile_commands.json';prefix='base-corpus' if side=='base' else 'tip-corpus';null=p/(prefix+'-null.jsonl');summary=p/(prefix+'-null.summary.json');outer=p/(prefix+'-outer.json')
  else:
   inv=json.loads((directory/'invocation.json').read_text());cmd=inv['command'];source=Path(cmd[cmd.index('--checkout')+1]);database=Path(cmd[cmd.index('--compile-database')+1]);null=directory/'null.jsonl';summary=directory/'null.summary.json';outer=directory/'outer.json'
  spec={'diagnostics':str(directory/'resource-diagnostics.json'),'contracts':str(directory/'resource-diagnostics.contracts.json'),'null':str(null),'null_summary':str(summary),'outer':str(outer),'manifest':str(p/'inputs/contract/profiles/fluent-bit/../../../cycle19-resource-contracts.json'),'database':str(database),'route':'pinned' if route=='corpus' else 'configured','source_root':str(source)}
  specpath=out/(route+'-'+side+'-projection-spec.json');specpath.write_text(json.dumps(spec,indent=2)+'\n');run(route+'-'+side+'-projection',[out/'projection-replay',specpath,out/(route+'-'+side+'-projection-proof.json')],env)
(out/'exit').write_text('0\n')
