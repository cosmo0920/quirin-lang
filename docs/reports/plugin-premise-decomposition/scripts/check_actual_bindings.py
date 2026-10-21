import argparse,hashlib,json,pathlib,subprocess,datetime
p=argparse.ArgumentParser();p.add_argument('route');a=p.parse_args();root=pathlib.Path.cwd();P=root/'private/plugin-premise-decomposition';report=root/'docs/reports/plugin-premise-decomposition';out=P/'actual-bindings'/a.route;out.mkdir(parents=True,exist_ok=False);env=json.loads((P/'route-environments'/a.route/'tip-environment.json').read_text());tip=P/'measure-1'/a.route/'tip';records=[]
for mode in ['resource','null']:
 directory=out/mode;directory.mkdir();sources={}
 if mode=='resource':
  for key,text in env['source_assumptions'].items():
   if any(key.endswith('/'+suffix) or key.endswith(':'+suffix) for suffix in ['librdkafka/client.qv','librdkafka/kafka_mem.qv','librdkafka/memory.qv','fluent_bit/logging.qv']):
    path=directory/(hashlib.sha256(key.encode()).hexdigest()+'.qv');path.write_text(text);sources[key]=str(path)
  assert len(sources)==4
 else:
  sources={'qcore-provider-librdkafka:librdkafka/null_deref.qv':str(P/'tip-freeze/inputs/providers/qcore-provider-librdkafka/contracts/librdkafka/null_deref.qv')}
 index=directory/'sources.json';index.write_text(json.dumps(sources,indent=2)+'\n')
 spec={'mode':mode,'source_index':str(index),'held_graphs':str(report/('bindings/'+mode+'-expected-graphs.json')),'held_manifest_index':str(P/'final-binding-inputs'/(mode+'-held-index.json')),'presented_manifest_index':str(P/'final-binding-inputs'/(mode+'-presented-index.json')),'artifact_index':str(P/'final-binding-inputs/artifacts.json'),'publication':str(tip/('resource-diagnostics.contracts.json' if mode=='resource' else a.route+'-null.jsonl'))}
 inp=directory/'input.json';inp.write_text(json.dumps(spec,indent=2)+'\n');command=[str(root/'target/debug/examples/plugin_sidecar_bindings'),str(inp),str(directory/'result.json')];start=datetime.datetime.now(datetime.timezone.utc).isoformat()
 with (directory/'command.log').open('wb') as stream:code=subprocess.run(command,stdout=stream,stderr=subprocess.STDOUT).returncode
 records.append({'mode':mode,'command':command,'exit':code,'started':start,'ended':datetime.datetime.now(datetime.timezone.utc).isoformat(),'input_sha256':hashlib.sha256(inp.read_bytes()).hexdigest(),'publication_sha256':hashlib.sha256(pathlib.Path(spec['publication']).read_bytes()).hexdigest()});(out/'commands.json').write_text(json.dumps(records,indent=2)+'\n');assert code==0
 result=json.loads((directory/'result.json').read_text());print(a.route,mode,result['matched_publications'],len(result['requirements']),flush=True)
