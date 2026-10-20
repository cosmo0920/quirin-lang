#!/usr/bin/env python3
"""Run the authoritative five-artifact diff without renaming public archives."""
import argparse,gzip,json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
p=argparse.ArgumentParser();p.add_argument('route');p.add_argument('base',type=Path);p.add_argument('tip',type=Path);p.add_argument('out',type=Path);a=p.parse_args()
private=ROOT/'private/premise-decomposition-cycle1/five-artifacts'/a.route;private.mkdir(parents=True,exist_ok=False)
names={'diagnostics':'resource-diagnostics.json','source':'monkey-source.json','chunk-contracts':'resource-diagnostics.chunk-contracts.json','source-effects':'resource-diagnostics.source-effects.json','contracts':'resource-diagnostics.contracts.json'}
for stem,name in names.items():
 path=a.base/name
 if path.exists():(private/(stem+'.json.gz')).write_bytes(gzip.compress(path.read_bytes(),mtime=0))
out=a.out.resolve();command=[sys.executable,str(ROOT/'docs/reports/corpus-repin/diff_artifacts.py'),'--archived',str(private),'--measured',str(a.tip.resolve()),'--output',str(out),'--full']
with (private/'command.log').open('wb') as log:code=subprocess.run(command,cwd=ROOT,stdout=log,stderr=subprocess.STDOUT).returncode
(private/'command.json').write_text(json.dumps({'command':command,'exit':code},indent=2)+'\n')
if code:raise SystemExit(code)
(private/'full-diff.json.gz').write_bytes(gzip.compress(out.read_bytes(),mtime=0));out.with_suffix(out.suffix+'.gz').write_bytes((private/'full-diff.json.gz').read_bytes());out.unlink()
