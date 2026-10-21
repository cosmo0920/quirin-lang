import argparse,json,pathlib,subprocess,hashlib
p=argparse.ArgumentParser();p.add_argument('route');a=p.parse_args();R=pathlib.Path.cwd();P=R/'private/plugin-premise-decomposition';out=P/'closed-world-facts'/a.route;out.mkdir(parents=True,exist_ok=False);commands=[];records={}
for side in ['base','tip']:
 d=P/'measure-1'/a.route/side;dest=out/(side+'.json');cmd=[str(R/'target/debug/examples/premise_outer_facts'),str(d/'resource-diagnostics.json'),str(d/'resource-diagnostics.contracts.json'),str(dest)]
 code=subprocess.call(cmd);commands.append(dict(command=cmd,exit=code));(out/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');assert code==0;records[side]=json.loads(dest.read_text())
assert records['base']==records['tip'];(out/'comparison.json').write_text(json.dumps({'route':a.route,'all_rows':len(records['base']),'full_closed_world_facts_and_verdict_records_equal':True,'base_sha256':hashlib.sha256((out/'base.json').read_bytes()).hexdigest(),'tip_sha256':hashlib.sha256((out/'tip.json').read_bytes()).hexdigest()},indent=2)+'\n');print(a.route,len(records['base']))
