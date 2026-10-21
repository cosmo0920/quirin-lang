#!/usr/bin/env python3
"""Reconstruct each actual route's exact lexical manifest and source environment.

The initial root-manifest preflight remains retained. All three actual routes
load the lexical profile/../../../ manifest; no source/path/hash is ignored.
"""
import json, subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
PRIVATE=ROOT/'private/premise-evidence-contracts'
sources={'system':'private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source','slice':'private/w47-tail/attempt-1/prep/fluent-bit-clang-source','corpus':'build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source'}
receipts=[]
for route,source in sources.items():
 out=PRIVATE/'route-environments'/route;out.mkdir(parents=True,exist_ok=False)
 rows=[]
 for side in ['base','tip']:
  command=[str(ROOT/'target/debug/examples/premise_environment'),str(PRIVATE/(side+'-inputs')/'contract/profiles/fluent-bit/../../../cycle19-resource-contracts.json'),'/home/cosmo/Documents/GitHub/qcore-c/'+source,str(out/(side+'-environment.json'))]
  with (out/(side+'-environment-hashes.json')).open('wb') as stream:code=subprocess.run(command,cwd=ROOT,stdout=stream).returncode
  row={'route':route,'side':side,'command':command,'exit':code};rows.append(row);receipts.append(row)
  (out/'commands.json').write_text(json.dumps(rows,indent=2)+'\n');assert code==0,row
(PRIVATE/'route-environments/commands.json').write_text(json.dumps(receipts,indent=2)+'\n')
