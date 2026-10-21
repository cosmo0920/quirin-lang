import datetime,fcntl,hashlib,json,os,pathlib,subprocess,sys
root=pathlib.Path.cwd();p=root/'private/premise-resolution-cycle2';g=p/'joint-gate-2';records=json.loads((g/'commands.json').read_text());assert len(records)==22 and all(r['exit']==0 for r in records);assert (g/'released.json').exists()
head=json.loads((g/'acquired.json').read_text())['head'];assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==head;assert not subprocess.check_output(['git','status','--porcelain'],text=True)
out=p/'postgate-source-1';out.mkdir();env=dict(os.environ,QCORE_AUDIT_BUILD_COMMIT=head,CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0');runs=[]
sha=lambda f:hashlib.file_digest(pathlib.Path(f).open('rb'),'sha256').hexdigest()
now=lambda:datetime.datetime.now(datetime.timezone.utc).isoformat()
def run(name,cmd):
 start=now();print('START',name,start,flush=True)
 with (out/(name+'.log')).open('w') as log:r=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT)
 runs.append({'name':name,'command':cmd,'cwd':str(root),'source':head,'started':start,'ended':now(),'exit':r.returncode,'log':str(out/(name+'.log')),'QCORE_AUDIT_BUILD_COMMIT':head});(out/'commands.json').write_text(json.dumps(runs,indent=2)+'\n');print('END',name,r.returncode,flush=True);assert r.returncode==0
with open('/tmp/qcore-c-card-validation-20261021.lock','a') as primary,open('/tmp/qcore-c-cycle2-small-compile.lock','a') as small:
 fcntl.flock(primary,fcntl.LOCK_EX);fcntl.flock(small,fcntl.LOCK_EX);(out/'acquired.json').write_text(json.dumps({'utc':now(),'source':head})+'\n')
 run('plugin-source',[str(root/'target/debug/examples/plugin_source_witnesses'),str(out/'plugin')])
 csrc=root/'private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source/lib/librdkafka-2.15.0/src'
 run('cjson-source',[str(root/'target/debug/examples/cjson_local_source'),str(csrc/'cJSON.c'),str(csrc/'cJSON.h'),str(out/'cjson')])
 binary=root/'target/release/quirin-audit';receipt={'source':head,'build':next(r for r in records if r['name']=='release'),'standing_gate_commands':str(g/'commands.json'),'binary':str(binary),'sha256':sha(binary),'meaning':'Default release binary actually built at the frozen successful joint gate source; both direct source examples also passed on that source.'};(out/'binary-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
 run('freeze-tip-inputs',[sys.executable,'docs/reports/premise-resolution-cycle2/scripts/freeze_inputs.py','private/premise-resolution-cycle2/tip-freeze','--source',head,'--binary',str(binary),'--binary-sha256',receipt['sha256'],'--binary-receipt',str(out/'binary-receipt.json')])
 run('compile-comparison-helpers',[sys.executable,'private/premise-resolution-cycle2/compile_comparator_helpers.py'])
 assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==head;assert not subprocess.check_output(['git','status','--porcelain'],text=True)
(out/'released.json').write_text(json.dumps({'utc':now(),'source':head})+'\n');print('SOURCE EXAMPLES/FREEZE/HELPERS COMPLETE',flush=True)
raise SystemExit(subprocess.run([sys.executable,str(p/'run_tip.py')],env=env).returncode)
