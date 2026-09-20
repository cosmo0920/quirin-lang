#!/usr/bin/env python3
"""Rehash changed native evidence, then require timeout semantic refusal."""
import argparse,copy,hashlib,json,pathlib,subprocess

def main():
 p=argparse.ArgumentParser();p.add_argument('--binary',type=pathlib.Path,required=True);p.add_argument('--config',type=pathlib.Path,required=True);p.add_argument('--output',type=pathlib.Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
 original=json.loads(a.config.read_text());results=[]
 for name in ['map-write-descriptor','event-cell-object','map-free-pointer','exit-thread','duplicate-sequence','role-substitution','report-alias']:
  out=(a.output/name).resolve();out.mkdir();c=copy.deepcopy(original);m=json.loads(pathlib.Path(c['run_manifest']).read_text());field='bindings'
  bindings=[json.loads(x) for x in pathlib.Path(c['bindings']).read_text().splitlines()];b=next(x for x in bindings if x.get('enter')==c['callback_enter_sequence'])
  if name=='map-write-descriptor':next(x for x in b['before'] if x['field']=='pipe[1]')['value']+=1
  elif name=='event-cell-object':next(x for x in b['before'] if x['field']=='event')['object']+=1
  elif name=='map-free-pointer':next(x for x in b['calls'] if x['function']=='mk_mem_free')['arguments'][0]+=1
  if name in ['exit-thread','duplicate-sequence']:
   field='observations';rows=[json.loads(x) for x in pathlib.Path(c[field]).read_text().splitlines()]
   if name=='exit-thread':next(x for x in rows if x['sequence']==b['exit'])['thread']+=1
   else:rows.append(rows[-1])
  else:rows=bindings
  if name not in ['role-substitution','report-alias']:
   selected=out/pathlib.Path(c[field]).name;data=('\n'.join(map(json.dumps,rows))+'\n').encode();selected.write_bytes(data);c[field]=str(selected);m['roles'][field]=str(selected);m['artifacts_sha256'][selected.name]=hashlib.sha256(data).hexdigest()
  if name=='role-substitution':c['observer_binary']=c['libevent_library']
  manifest=out/'manifest.json';manifest.write_text(json.dumps(m));c['run_manifest']=str(manifest);c['report']=str(out/'report.json') if name!='report-alias' else c['observations']
  before=pathlib.Path(c['observations']).read_bytes();config=out/'config.json';config.write_text(json.dumps(c))
  run=subprocess.run([str(a.binary.resolve()),'timeout-teardown',str(config)],capture_output=True,text=True)
  (out/'audit.log').write_text(run.stdout+run.stderr);row={'control':name,'exit':run.returncode,'reason':run.stderr.strip(),'observations_unchanged':pathlib.Path(c['observations']).read_bytes()==before};results.append(row)
  if run.returncode!=1 or not row['observations_unchanged']:raise RuntimeError(row)
 (a.output/'results.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps(results,indent=2))
if __name__=='__main__':main()
