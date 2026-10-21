import copy,hashlib,json,os,pathlib,subprocess,datetime
P=pathlib.Path('private/plugin-premise-decomposition').resolve();out=P/'reading-negative-controls';out.mkdir(exist_ok=False);spec=json.loads((P/'reading-proof/slice/attempt6/input.json').read_text());original=json.loads(pathlib.Path(spec['tip_diagnostics']).read_text());ui=next(i for i,u in enumerate(original['units']) if any(r.get('reading_site_evidence') for r in u['functions']));ri=next(i for i,r in enumerate(original['units'][ui]['functions']) if r.get('reading_site_evidence'));envsha=original['units'][ui]['functions'][ri]['reading_site_evidence'][0]['observation']['environment_sha256'];receipts=[]
for kind in ['wrong-build-context','wrong-property-context','missing-effective-environment','noncanonical-effective-environment','tampered-effective-environment']:
 directory=out/kind;directory.mkdir();data=copy.deepcopy(original);unit=data['units'][ui];row=unit['functions'][ri]
 if kind=='wrong-build-context':row['hypothesis_context']['build_identity']='0'*64
 if kind=='wrong-property-context':row['hypothesis_context']['property']='c.null.deref.v1'
 if kind=='missing-effective-environment':del unit['reading_site_environments']['environments'][envsha]
 if kind=='noncanonical-effective-environment':unit['reading_site_environments']['environments'][envsha]+='\n'
 if kind=='tampered-effective-environment':
  v=json.loads(unit['reading_site_environments']['environments'][envsha]);v['source_assumptions']['forged']='forged';unit['reading_site_environments']['environments'][envsha]=json.dumps(v,separators=(',',':'))
 presented=directory/'diagnostics.json';presented.write_text(json.dumps(data,separators=(',',':')));s=dict(spec);s['tip_diagnostics']=str(presented);inp=directory/'input.json';inp.write_text(json.dumps(s,indent=2)+'\n');cmd=[str(P/'attribute-reading-environments-v5'),str(inp),str(directory/'should-not-exist.json')];start=datetime.datetime.now(datetime.timezone.utc).isoformat()
 with (directory/'command.log').open('wb') as log:code=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env={**os.environ,'QCORE_CLANG':'/usr/lib/llvm-21/bin/clang'}).returncode
 assert code!=0 and not (directory/'should-not-exist.json').exists();receipts.append({'name':kind,'actual_exit':code,'expected':'refuse exact one-fact change','command':cmd,'started':start,'ended':datetime.datetime.now(datetime.timezone.utc).isoformat(),'presented_sha256':hashlib.sha256(presented.read_bytes()).hexdigest(),'disposition_created':False});(out/'commands.json').write_text(json.dumps(receipts,indent=2)+'\n')
print('5 exact-context/canonical-effective-environment negative controls refused')
