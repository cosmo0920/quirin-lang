#!/usr/bin/env python3
"""Six fresh audits; configured pairs at 8 workers, corpus at16; caller owns flock."""
import concurrent.futures,hashlib,json,subprocess,sys,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
P=ROOT/'private/premise-evidence-contracts'
# Each supplied snapshot includes explicit root manifest and immutable binary.
for side in ['base','tip']:
 snapshot=P/(side+'-inputs')
 assert (snapshot/'cycle19-resource-contracts.json').is_file()
 assert (snapshot/'target/release/quirin-audit').is_file()
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
def run(route,side):
 out=P/'measure-1'/route/side
 command=[sys.executable,str(Path(__file__).with_name('measure.py')),route,str(out),'--link-map',str(ROOT/'docs/reports/premise-evidence-contracts/link-map.corpus.json'),'--inputs-snapshot',str(P/(side+'-inputs')),'--binary-source','c5c298db2ab7dedb5d39d29f4cfd097727e267b2' if side=='base' else head,'--jobs','16' if route=='corpus' else '8']
 code=subprocess.call(command,cwd=ROOT)
 if route=='corpus' and code==0:
  for f in (out/'corpus').iterdir():
   if f.is_file() and f.suffix in ['.json','.jsonl'] and not (out/f.name).exists():
    (out/f.name).symlink_to(f.relative_to(out))
 print(json.dumps(dict(route=route,side=side,exit=code)),flush=True)
 return code
for side in ['base','tip']:
 with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
  codes=list(pool.map(lambda route:run(route,side),['system','slice']))
 if any(codes):raise SystemExit(1)
for side in ['base','tip']:
 if run('corpus',side):raise SystemExit(1)
