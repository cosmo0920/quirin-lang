"""Run the standing gate through Lean, recording direct exits and private logs.

Run from the worktree root with a fresh run-directory basename. The integrator
then publishes gate.json, regenerates the matrix, and runs scripts/tests;
Python must be after regeneration and is deliberately a separate recorded step.
"""
import datetime, json, os, pathlib, re, subprocess, sys, time
root=pathlib.Path.cwd(); out=root/'private/b-series'/sys.argv[1]; out.mkdir(parents=True,exist_ok=False)
env=dict(os.environ,CARGO_BUILD_JOBS='3'); facts={'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'start':datetime.datetime.now(datetime.timezone.utc).isoformat(),'cwd':str(root),'environment':{k:env.get(k) for k in ['CARGO_BUILD_JOBS','QCORE_CLANG','RUSTFLAGS','CARGO_TARGET_DIR']},'steps':[]}
commands=[('submodules',['git','submodule','update','--init','--recursive']),('clang-version',['clang','--version']),('cargo-test',['cargo','test','--workspace','--no-fail-fast']),('cargo-fmt',['cargo','fmt','--all','--','--check']),('cargo-clippy',['cargo','clippy','--all-targets','--','-D','warnings']),('reevaluation-probe',['cargo','run','-p','qcore-compiler','--example','reevaluation_probe']),('reevaluation-followup',['cargo','run','-p','qcore-compiler','--example','reevaluation_followup']),('lean-audits',['python3','scripts/check_lean_audits.py','--jobs','2'])]
for name,argv in commands:
 start=time.monotonic(); print('Starting',name,flush=True)
 with (out/(name+'.log')).open('w') as log: code=subprocess.call(argv,stdout=log,stderr=subprocess.STDOUT,env=env)
 row={'name':name,'command':argv,'exit':code,'seconds':round(time.monotonic()-start,2),'log':str(out/(name+'.log'))}; facts['steps'].append(row); (out/(name+'.exit')).write_text(str(code)+'\n'); (out/'gate-facts.json').write_text(json.dumps(facts,indent=2)+'\n'); print(row,flush=True)
 if name=='submodules' and code: raise SystemExit(code)
text=(out/'cargo-test.log').read_text(); counts=re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
facts['rust']={'passed':sum(int(x[1]) for x in counts),'failed':sum(int(x[2]) for x in counts),'ignored':sum(int(x[3]) for x in counts),'result_summaries':len(counts),'running_targets':len(re.findall(r'^\s*Running ',text,re.M)),'doc_targets':len(re.findall(r'^\s*Doc-tests ',text,re.M))}; facts['end']=datetime.datetime.now(datetime.timezone.utc).isoformat(); (out/'gate-facts.json').write_text(json.dumps(facts,indent=2)+'\n'); print(json.dumps(facts,indent=2),flush=True); raise SystemExit(any(x['exit'] for x in facts['steps']))
