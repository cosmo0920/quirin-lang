import argparse,gzip,json,pathlib,subprocess,hashlib
p=argparse.ArgumentParser();p.add_argument('route');a=p.parse_args();R=pathlib.Path.cwd();P=R/'private/plugin-premise-decomposition';out=P/'five-artifact-diff'/a.route;out.mkdir(parents=True,exist_ok=False);base=P/'measure-1'/a.route/'base';tip=P/'measure-1'/a.route/'tip';archive=out/'archived';measured=out/'measured';archive.mkdir();measured.mkdir();transports=[]
names={'diagnostics':'resource-diagnostics.json','source':'monkey-source.json','chunk-contracts':'resource-diagnostics.chunk-contracts.json','source-effects':'resource-diagnostics.source-effects.json','contracts':'resource-diagnostics.contracts.json'}
for stem,name in names.items():
 for side,source in [('base',base/name),('tip',tip/name)]:
  if not source.exists():continue
  raw=source.read_bytes();transport=raw
  if stem=='contracts':
   value=json.loads(raw)
   if 'sources' not in value:
    assert 'contracts' in value;value=dict(value);value['sources']=value.pop('contracts');transport=(json.dumps(value,indent=2)+'\n').encode();transports.append({'side':side,'source_sha256':hashlib.sha256(raw).hexdigest(),'adapter_sha256':hashlib.sha256(transport).hexdigest(),'operation':'rename existing top-level contracts source map to sources for legacy tool, all original fields retained in raw comparison'})
  (archive/(stem+'.json.gz')).write_bytes(gzip.compress(transport,mtime=0)) if side=='base' else (measured/name).write_bytes(transport)
command=['python3','-B','docs/reports/corpus-repin/diff_artifacts.py','--archived',str(archive),'--measured',str(measured),'--output',str(out/'diff.json')]
with (out/'command.log').open('wb') as stream:code=subprocess.run(command,stdout=stream,stderr=subprocess.STDOUT).returncode
(out/'command.json').write_text(json.dumps({'command':command,'exit':code,'explicit_transport':transports},indent=2)+'\n');assert code==0
x=json.loads((out/'diff.json').read_text());assert x['schema_version']==2
print(a.route,'official five artifact diff exit0')
