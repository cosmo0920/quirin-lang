#!/usr/bin/env python3
"""Exercise payload audit semantic refusals with producer hashes recomputed."""
import argparse, copy, hashlib, json, pathlib, subprocess

def sha(data):
    return hashlib.sha256(data).hexdigest()

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--binary',type=pathlib.Path,required=True)
    p.add_argument('--config',type=pathlib.Path,required=True)
    p.add_argument('--output',type=pathlib.Path,required=True)
    a=p.parse_args(); a.output.mkdir(parents=True,exist_ok=False)
    original=json.loads(a.config.read_text())
    results=[]
    for name in ['missing-allocation-field','allocation-extent','wrong-storage-authority','missing-write-field','callback-thread','duplicate-binding','missing-path','backend','role-substitution','extra-execution','report-alias']:
        out=(a.output/name).resolve();out.mkdir()
        c=copy.deepcopy(original);m=json.loads(pathlib.Path(c['run_manifest']).read_text())
        def replace(field,data):
            selected=out/pathlib.Path(c[field]).name;selected.write_bytes(data)
            c[field]=str(selected);m['files'][str(selected)]=sha(data);m['roles'][field]=str(selected)
        if name in ['missing-allocation-field','allocation-extent']:
            v=json.loads(pathlib.Path(c['allocation']).read_text())
            if name=='missing-allocation-field': del v['context_allocation_bytes']
            else: v['fired_allocation_bytes']-=1
            replace('allocation',json.dumps(v).encode())
        elif name=='wrong-storage-authority':c['authority']['context_object']+=1
        elif name=='missing-write-field':c['authority']['writable_fired_fields'].remove('mask')
        elif name=='callback-thread':
            rows=[json.loads(s) for s in pathlib.Path(c['observations']).read_text().splitlines()]
            next(r for r in rows if r['action'].get('operation')=='callback_exit')['thread']+=1
            replace('observations',('\n'.join(map(json.dumps,rows))+'\n').encode())
        elif name in ['duplicate-binding','missing-path']:
            rows=[json.loads(s) for s in pathlib.Path(c['bindings']).read_text().splitlines()]
            if name=='duplicate-binding':rows.append(next(r for r in rows if r['kind']=='callback'))
            else: rows.pop()
            replace('bindings',('\n'.join(map(json.dumps,rows))+'\n').encode())
        elif name=='role-substitution':
            c['observer_binary']=c['libevent_library']
        elif name=='extra-execution':
            rows=[json.loads(s) for s in pathlib.Path(c['bindings']).read_text().splitlines()]
            b=copy.deepcopy(next(r for r in reversed(rows) if r['kind']=='callback'))
            observations=[json.loads(s) for s in pathlib.Path(c['observations']).read_text().splitlines()]
            new_enter=copy.deepcopy(next(r for r in observations if r['sequence']==b['enter']))
            new_exit=copy.deepcopy(next(r for r in observations if r['sequence']==b['exit']))
            high=max(r['sequence'] for r in observations)
            new_enter['sequence']=high+1;new_exit['sequence']=high+2
            b['enter']=high+1;b['exit']=high+2;rows.append(b);observations.extend([new_enter,new_exit])
            replace('bindings',('\n'.join(map(json.dumps,rows))+'\n').encode())
            replace('observations',('\n'.join(map(json.dumps,observations))+'\n').encode())
        elif name=='backend':
            v={'backend':'unsupported','dispatcher_count':1};m['runtime']=v
            replace('runtime',json.dumps(v).encode())
        manifest=out/'manifest.json';manifest.write_text(json.dumps(m));c['run_manifest']=str(manifest)
        c['report']=str(out/'report.json') if name!='report-alias' else c['observations']
        before=pathlib.Path(c['observations']).read_bytes()
        config=out/'config.json';config.write_text(json.dumps(c))
        run=subprocess.run([str(a.binary.resolve()),'event-payload',str(config)],capture_output=True,text=True)
        (out/'stdout.log').write_text(run.stdout);(out/'stderr.log').write_text(run.stderr)
        row={'control':name,'exit':run.returncode,'reason':run.stderr.strip(),'observations_unchanged':pathlib.Path(c['observations']).read_bytes()==before}
        results.append(row)
        if run.returncode!=1 or not row['observations_unchanged']:raise RuntimeError(row)
    (a.output/'results.json').write_text(json.dumps(results,indent=2)+'\n')
    print(json.dumps(results,indent=2))
if __name__=='__main__':main()
