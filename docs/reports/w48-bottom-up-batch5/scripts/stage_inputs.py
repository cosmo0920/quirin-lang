#!/usr/bin/env python3
"""Byte-copy completed native outputs into one private flat comparison input."""
import argparse,hashlib,json
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('route',choices=['system','slice','corpus']);p.add_argument('base',type=Path);p.add_argument('tip',type=Path);p.add_argument('out',type=Path);a=p.parse_args();assert a.out.is_absolute() and '/private/' in str(a.out);a.out.mkdir(parents=True,exist_ok=False);sha=lambda b:hashlib.sha256(b).hexdigest()
names=['invocation.json','resource-diagnostics.json','resource-diagnostics.contracts.json','resource-diagnostics.chunk-contracts.json','resource-diagnostics.source-effects.json','monkey-source.json',a.route+'-null.jsonl',a.route+'-null.summary.json',a.route+'-outer.json','run.json','completion.json']
for side,directory in [('base',a.base.resolve()),('tip',a.tip.resolve())]:
 inv=json.loads((directory/'invocation.json').read_bytes());assert inv['exit']==0 and 'identity_error'not in inv
 dest=a.out/side;dest.mkdir();records=[]
 for name in names:
  candidates=[x for x in [directory/name,directory/'corpus'/name] if x.is_file()];assert len(candidates)<=1,(side,name,'ambiguous artifact')
  if not candidates:
   assert name not in ['invocation.json','resource-diagnostics.json','resource-diagnostics.contracts.json',a.route+'-null.jsonl',a.route+'-outer.json'];continue
  source=candidates[0];raw=source.read_bytes();target=dest/name;target.write_bytes(raw);assert target.read_bytes()==source.read_bytes();records.append({'source':str(source),'copy':str(target),'sha256':sha(raw),'bytes':len(raw)})
 for r in records:assert sha(Path(r['source']).read_bytes())==sha(Path(r['copy']).read_bytes())==r['sha256']
 (dest/'copy-manifest.json').write_text(json.dumps(records,indent=2)+'\n')
print(json.dumps({'route':a.route,'out':str(a.out),'byte_exact_private_copies':True}))
