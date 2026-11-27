#!/usr/bin/env python3
"""Count graph publication in a measured route (metadata, not safety coverage)."""
import argparse,gzip,json
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('comparison',type=Path);p.add_argument('route');a=p.parse_args();d=a.comparison/'after'
def load(name):return json.loads(gzip.decompress((d/(name+'.gz')).read_bytes()))
u=load('resource-diagnostics.json');c=load('resource-diagnostics.contracts.json');rows=[f for unit in u['units'] for f in unit['functions']];decl=c['contract_dependencies']['declarations'];null=[json.loads(line) for line in gzip.decompress((d/(a.route+'-null.jsonl.gz')).read_bytes()).splitlines()]
r={'route':a.route,'custody_rows':len(rows),'custody_rows_with_context':sum('hypothesis_context' in f for f in rows),'custody_rows_with_atom_closure':sum('hypothesis_dependencies' in f.get('contract_dependencies',{}) for f in rows),'declarations':len(decl),'declarations_with_graph':sum('hypothesis_graph' in x for x in decl.values()),'null_rows':len(null),'null_rows_with_graph':sum('hypothesis_graph' in x for x in null),'meaning':'metadata publication counts; no proposition truth or additional safety coverage'}
assert r['custody_rows']==r['custody_rows_with_context']==r['custody_rows_with_atom_closure'];assert r['declarations']==r['declarations_with_graph'];assert r['null_rows']==r['null_rows_with_graph'];(a.comparison/'publication-counts.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r))
