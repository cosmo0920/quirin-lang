#!/usr/bin/env python3
"""Run the authoritative five-artifact diff without renaming public archives."""
import argparse,gzip,json,subprocess,sys,runpy
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

# The configured route calls its retained text map contracts, whereas the
# pinned route and historical itemizer call it sources. Adapt only that field
# into the official source-key normalizer; retain all raw sidecar keys above.
left=json.loads((a.base/'resource-diagnostics.contracts.json').read_text())
right=json.loads((a.tip/'resource-diagnostics.contracts.json').read_text())
tool=runpy.run_path(str(ROOT/'docs/reports/corpus-repin/diff_artifacts.py'))
field=lambda doc:'contracts' if isinstance(doc.get('contracts'),dict) else 'sources'
module_diff=tool['diff_contracts']({'sources':left[field(left)]},{'sources':right[field(right)]},True)
record={'actual_source_fields':{'base':field(left),'tip':field(right)},'meaning':'Official contract source-key normalizer and itemizer; raw keys retained, collision behavior unchanged; complete original five-artifact diff retained separately.','diff':module_diff}
out.with_name('contract-source-keys.json.gz').write_bytes(gzip.compress((json.dumps(record,indent=2)+'\n').encode(),mtime=0))
