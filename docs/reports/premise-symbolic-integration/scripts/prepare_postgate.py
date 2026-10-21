import hashlib,json,pathlib,subprocess
root=pathlib.Path.cwd();p=root/'private/premise-symbolic-integration';gate=p/'joint-gate-3';out=p/'postgate-source-1';out.mkdir(exist_ok=False)
records=json.loads((gate/'commands.json').read_bytes());source=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
assert (gate/'released.json').exists() and len(records)==22 and len({r['name'] for r in records})==22
assert all(r['exit']==0 and r['source']==source for r in records)
assert not (gate/'head-guard-failure.json').exists()
completion=json.loads((gate/'completion.json').read_bytes());assert completion=={'wrapper_exit':0,'source':source,'head_after':source,'clean_after':True,'all_commands_zero':True,'command_count':22,'commands_sha256':hashlib.sha256((gate/'commands.json').read_bytes()).hexdigest()}
assert not subprocess.check_output(['git','status','--porcelain'],text=True)
binary=root/'target/release/quirin-audit';digest=hashlib.sha256(binary.read_bytes()).hexdigest();build=[r for r in records if r['name']=='release'];assert len(build)==1
receipt={'source':source,'build':build[0],'standing_gate_commands':str(gate/'commands.json'),'binary':str(binary),'sha256':digest,'meaning':'Default release binary actually built at the frozen all22green joint gate source. Both source example builds pass on the same source.'};rp=out/'binary-receipt.json';rp.write_text(json.dumps(receipt,indent=2)+'\n')
command=['python3','docs/reports/premise-symbolic-integration/scripts/freeze_inputs.py',str(p/'tip-freeze'),'--source',source,'--binary',str(binary),'--binary-sha256',digest,'--binary-receipt',str(rp)]
with (out/'freeze.log').open('w') as log:r=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
(out/'commands.json').write_text(json.dumps([{'command':command,'exit':r.returncode}],indent=2)+'\n');assert r.returncode==0
assert hashlib.sha256(binary.read_bytes()).hexdigest()==digest
print(json.dumps({'source':source,'binary_sha256':digest,'freeze':str(p/'tip-freeze'),'gate_all22':True}))
