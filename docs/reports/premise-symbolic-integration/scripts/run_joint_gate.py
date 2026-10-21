import datetime,fcntl,json,os,pathlib,subprocess,sys,time
root=pathlib.Path.cwd();out=root/'private/premise-symbolic-integration'/sys.argv[1];assert not out.exists();out.mkdir(parents=True)
head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();assert not subprocess.check_output(['git','status','--porcelain'],text=True)
env=os.environ.copy();overrides={'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0','QCORE_AUDIT_BUILD_COMMIT':head};env.update(overrides)
packages=out/'provider/packages'
commands=[('submodules',['git','submodule','update','--init','--recursive']),('clang',['clang','--version']),('rust',['python3','scripts/run_test_binaries.py','--jobs','4']),('fmt',['cargo','fmt','--all','--','--check']),('clippy',['cargo','clippy','--all-targets','--','-D','warnings']),('probe',['cargo','run','-p','qcore-compiler','--example','reevaluation_probe']),('followup',['cargo','run','-p','qcore-compiler','--example','reevaluation_followup']),('lean',['python3','scripts/check_lean_audits.py']),('provider-rust',['cargo','test','--manifest-path','providers/Cargo.toml','--workspace','--locked','--offline','--no-fail-fast']),('provider-fmt',['cargo','fmt','--manifest-path','providers/Cargo.toml','--all','--','--check']),('provider-clippy',['cargo','clippy','--manifest-path','providers/Cargo.toml','--workspace','--all-targets','--locked','--offline','--','-D','warnings']),('provider-python',['python3','-m','unittest','discover','-s','providers/tests']),('provider-contracts',['python3','providers/contracts.py','check']),('package-build',['python3','providers/package.py','build','--output',str(packages)])]
commands += [('package-verify-'+p,['python3','providers/package.py','verify',str(packages/('qcore.'+p))]) for p in ['chunkio','libevent','librdkafka','cmetrics','ctraces']]
commands += [('audit-examples',['cargo','build','-p','qcore-audit','--examples']),('source-examples',['cargo','build','-p','quirin-source-analysis','--examples']),('release',['cargo','build','--release','-p','qcore-audit','--bin','quirin-audit'])]
records=[]
def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
with open('/tmp/qcore-c-card-validation-20261021.lock','a') as lock, open('/tmp/qcore-c-cycle2-small-compile.lock','a') as small:
 print('WAITING FOR SERIAL VALIDATION LOCKS',flush=True)
 fcntl.flock(lock,fcntl.LOCK_EX);fcntl.flock(small,fcntl.LOCK_EX);(out/'acquired.json').write_text(json.dumps({'utc':now(),'head':head},indent=2)+'\n')
 for name,cmd in commands:
  start=time.monotonic();started=now();print('START',name,started,flush=True)
  with (out/(name+'.log')).open('w') as log:p=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
  r=dict(name=name,command=cmd,cwd=str(root),source=head,started=started,ended=now(),seconds=time.monotonic()-start,exit=p.returncode,environment_overrides=overrides,log=str(out/(name+'.log')));records.append(r);(out/(name+'.exit')).write_text(str(p.returncode)+'\n');(out/'commands.json').write_text(json.dumps(records,indent=2)+'\n');print('END',name,p.returncode,flush=True)
 # Run every step, retaining all failures. Native TIP never uses an unsuccessful gate.
(out/'released.json').write_text(json.dumps({'utc':now()},indent=2)+'\n');assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==head
print('SYMBOLIC INTEGRATION GATE COMPLETE',json.dumps({r['name']:r['exit'] for r in records}),flush=True)
raise SystemExit(int(any(r['exit'] for r in records)))
