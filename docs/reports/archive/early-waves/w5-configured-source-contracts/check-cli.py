import json,os,subprocess,pathlib
root=pathlib.Path.cwd();exe=root/'target/debug/examples/cycle19_fluent_bit_libraries_audit'
cases=[('missing_config',{'QCORE_SOURCE_CONTRACTS':'/qcore-missing-source-contract-config.json'},'source-contract configuration'),('empty_legacy',{'QCORE_CHUNKIO_PACKAGE':''},'cannot be empty'),('missing_legacy_package',{'QCORE_CHUNKIO_PACKAGE':'/qcore-missing-source-provider'},'source-contract package'),('conflicting_selectors',{'QCORE_CHUNKIO_PACKAGE':'unused','QCORE_SOURCE_CONTRACTS':'unused'},'cannot both select')]
records=[]
for name,env,expected in cases:
 inherited=os.environ.copy()
 for key in ['QCORE_SOURCE_CONTRACTS','QCORE_CHUNKIO_PACKAGE']: inherited.pop(key,None)
 inherited.update(env)
 result=subprocess.run([str(exe),'/qcore-missing-checkout'],env=inherited,capture_output=True,text=True,timeout=30)
 assert result.returncode!=0 and expected in result.stderr,(name,result)
 records.append(dict(case=name,environment=env,exit_code=result.returncode,stdout=result.stdout,stderr=result.stderr))
print(json.dumps(records,indent=2))
