import argparse,collections,hashlib,json,pathlib
a=argparse.ArgumentParser();a.add_argument("--routes",default="system,slice,corpus");a=a.parse_args();selected=a.routes.split(",");assert selected and len(set(selected))==len(selected) and set(selected)<=set(["system","slice","corpus"])
p=pathlib.Path('private/premise-resolution-cycle2');out=p/'prepared-tip';out.mkdir(exist_ok=True);summary=[]
sha=lambda f:hashlib.file_digest(f.open('rb'),'sha256').hexdigest()
for route in selected:
 invpath=p/'measure-1'/route/'tip/invocation.json';inv=json.loads(invpath.read_text());assert inv['exit']==0;assert inv['binary_sha256']==inv['binary_sha256_after'];assert inv['inputs_before']==inv['inputs_after'];assert inv['head']==inv['head_after'];assert inv['source_identity']==inv['source_identity_after'];directory=out/route;directory.mkdir(exist_ok=True);manifest=[]
 for r in json.loads((p/'prepared-base'/route/'copy-manifest.json').read_text()):
  assert r['source'].count('/base/')==1
  source=pathlib.Path(r['source'].replace('/base/','/tip/'));target=directory/pathlib.Path(r['copy']).name;assert source.is_file();assert not target.exists() or source.read_bytes()==target.read_bytes(),('existing prepared copy differs',str(target));target.write_bytes(source.read_bytes());assert sha(source)==sha(target);manifest.append({'source':str(source),'copy':str(target),'sha256':sha(source)})
 assert not (directory/'copy-manifest.json').exists() or json.loads((directory/'copy-manifest.json').read_text())==manifest,('existing manifest differs',route)
 (directory/'copy-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');data=json.loads((directory/'resource-diagnostics.json').read_text());rows=[r for u in data['units'] for r in u['functions']];counts=dict(collections.Counter(r['status'] for r in rows));summary.append({'route':route,'units':len(data['units']),'rows':len(rows),'counts':counts,'invocation':str(invpath),'source_guard_equal':True,'byte_exact_prepared_view':str(directory),'exit':inv['exit']})
(p/('tip-route-summary-'+('-'.join(selected))+'.json')).write_text(json.dumps({'routes':summary,'meaning':'All copied bytes checked equal; no artifact normalization or field changes.'},indent=2)+'\n');print(json.dumps(summary))
